use base64ct::Encoding;
use email_address::EmailAddress;
use smtp_proto::{
    AUTH_PLAIN, EXT_8BIT_MIME, EXT_AUTH, EXT_ENHANCED_STATUS_CODES, EXT_SMTP_UTF8, EhloResponse,
    Request,
};
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use tracing::{debug, trace};

use crate::models::{NewMessage, SmtpCredential, SmtpCredentialRepository};

pub struct SmtpSession {
    queue: Sender<NewMessage>,
    smtp_credentials: SmtpCredentialRepository,

    peer_addr: SocketAddr,
    peer_name: Option<String>,
    authenticated_credential: Option<SmtpCredential>,
    current_message: Option<NewMessage>,
}

pub enum SessionReply {
    ReplyAndContinue(u16, String),
    ReplyAndStop(u16, String),
    RawReply(Vec<u8>),
    IngestData(u16, String),
    IngestAuth(u16, String),
}

pub enum DataReply {
    ReplyAndContinue(u16, String),
    ContinueIngest,
}

struct AttemptedAuth<'a> {
    username: &'a str,
    password: &'a str,
}

enum AttemptedAuthError {
    SyntaxError,
    Utf8Error,
}

impl SmtpSession {
    const MAX_BODY_SIZE: u64 = 20 * 1024 * 1024;

    const RESPONSE_OK: &str = "2.0.0 Ok";
    const RESPONSE_FROM_OK: &str = "2.1.0 Originator <[email]> ok";
    const RESPONSE_TO_OK: &str = "2.1.5 Recipient <[email]> ok";
    const RESPONSE_SYNTAX_ERROR: &str = "5.5.2 Syntax error";
    const RESPONSE_AUTH_SUCCCESS: &str = "2.7.0 Authentication succeeded.";
    const RESPONSE_START_DATA: &str = "3.5.4 Start mail input; end with <CRLF>.<CRLF>";
    const RESPONSE_BYE: &str = "2.0.0 Goodbye";
    const RESPONSE_MESSAGE_ACCEPTED: &str = "2.6.0 Message queued for delivery";
    const RESPONSE_MESSAGE_REJECTED: &str = "5.6.0 Message rejected";
    const RESPONSE_BAD_SEQUENCE: &str = "5.5.1 Bad sequence of commands";
    const RESPONSE_MAIL_FIRST: &str = "5.5.1 Use MAIL first";
    const RESPONSE_HELLO_FIRST: &str = "5.5.1 Be nice and say EHLO first";
    const RESPONSE_NOVALID_RECIPIENTS: &str = "5.5.1 No valid recipients";
    const RESPONSE_INVALID_SENDER: &str = "5.1.7 This sender address is not valid";
    const RESPONSE_INVALID_EMAIL: &str = "5.1.3 This email address is not valid";
    const RESPONSE_NESTED_MAIL: &str = "5.5.1 Error: nested MAIL command";
    const RESPONSE_ALREADY_AUTHENTICATED: &str = "5.5.1 Already authenticated";
    const RESPONSE_AUTH_ERROR: &str = "5.7.8 Authentication credentials invalid";
    const RESPONSE_AUTHENTICATION_REQUIRED: &str = "5.7.1 Authentication required";
    const RESPONSE_ALREADY_TLS: &str = "5.7.4 Already in TLS mode";
    const RESPONSE_COMMAND_NOT_IMPLEMENTED: &str = "5.5.1 Command not implemented";
    const RESPONSE_MUST_USE_ESMTP: &str = "5.5.1 Must use EHLO";
    const RESPONSE_NO_VRFY: &str = "5.5.1 VRFY command is disabled";

    pub fn new(
        peer_addr: SocketAddr,
        queue: Sender<NewMessage>,
        user_repository: SmtpCredentialRepository,
    ) -> Self {
        Self {
            queue,
            smtp_credentials: user_repository,
            peer_addr,
            peer_name: None,
            current_message: None,
            authenticated_credential: None,
        }
    }

    pub fn peer(&self) -> &SocketAddr {
        &self.peer_addr
    }

    pub async fn handle(
        &mut self,
        request: Result<Request<String>, smtp_proto::Error>,
    ) -> SessionReply {
        let request = match request {
            Ok(r) => r,
            Err(e) => {
                debug!("failed to parse request: {e}");

                // RFC 4409, 4.1
                return SessionReply::ReplyAndContinue(554, e.to_string());
            }
        };

        if let Request::Auth { mechanism, .. } = request {
            // This is a workaround as we are not in control of the `Debug` implementation of `Request`
            // Without this if statement, we would print the user password as base64 string in the logs
            // which we want to avoid
            trace!(
                "received AUTH with mechanism {mechanism} request from {}",
                self.peer_addr
            );
        } else {
            trace!("received request: {request:?} from {}", self.peer_addr);
        }

        match request {
            Request::Ehlo { host } => {
                // RFC5231, 4.1.1.1
                let mut response = EhloResponse::new(&host);
                response.capabilities =
                    EXT_ENHANCED_STATUS_CODES | EXT_8BIT_MIME | EXT_SMTP_UTF8 | EXT_AUTH;

                response.auth_mechanisms = AUTH_PLAIN;

                let mut buf = Vec::with_capacity(64);
                response.write(&mut buf).ok();

                self.peer_name = Some(host);

                SessionReply::RawReply(buf)
            }
            Request::Lhlo { host: _ } => {
                // we do not currently support LMTP
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into())
            }
            Request::Helo { host: _ } => {
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_MUST_USE_ESMTP.into())
            }
            Request::Auth {
                mechanism,
                initial_response,
            } => {
                // RFC 4954
                if self.authenticated_credential.is_some() {
                    return SessionReply::ReplyAndContinue(
                        503,
                        Self::RESPONSE_ALREADY_AUTHENTICATED.into(),
                    );
                }

                if mechanism == AUTH_PLAIN {
                    debug!("Received AUTH PLAIN");

                    if initial_response.is_empty() {
                        return SessionReply::IngestAuth(334, "Tell me your secret.".into());
                    }

                    let (code, message) = self
                        .handle_plain_auth(&mut initial_response.into_bytes())
                        .await;

                    SessionReply::ReplyAndContinue(code, message)
                } else {
                    // other authentication methods
                    debug!("Received unsupported AUTH request");
                    SessionReply::ReplyAndContinue(535, Self::RESPONSE_AUTH_ERROR.into())
                }
            }
            Request::Quit => {
                // RFC5321, 4.1.1.10
                SessionReply::ReplyAndStop(221, Self::RESPONSE_BYE.into())
            }
            // if the client did not say EHLO, we want to ask for that first instead of processing any of the below commands
            _ignored_command if self.peer_name.is_none() => {
                SessionReply::ReplyAndContinue(503, Self::RESPONSE_HELLO_FIRST.into())
            }
            Request::Mail { from } => {
                // RFC5231, 4.1.1.2
                debug!("received MAIL FROM: {}", from.address);

                let Ok(from_address) = from.address.parse::<EmailAddress>() else {
                    return SessionReply::ReplyAndContinue(
                        553,
                        Self::RESPONSE_INVALID_SENDER.into(),
                    );
                };

                let Some(credential) = self.authenticated_credential.as_ref() else {
                    return SessionReply::ReplyAndContinue(
                        530,
                        Self::RESPONSE_AUTHENTICATION_REQUIRED.into(),
                    );
                };

                if self.current_message.is_some() {
                    return SessionReply::ReplyAndContinue(503, Self::RESPONSE_NESTED_MAIL.into());
                }

                self.current_message = Some(NewMessage::new(credential.id(), from_address));

                let response_message = Self::RESPONSE_FROM_OK.replace("[email]", &from.address);
                SessionReply::ReplyAndContinue(250, response_message)
            }
            Request::Rcpt { to } => {
                // RFC5231, 4.1.1.3
                debug!("received RCPT TO: {}", to.address);

                let Ok(to_address) = to.address.parse::<EmailAddress>() else {
                    return SessionReply::ReplyAndContinue(
                        553,
                        Self::RESPONSE_INVALID_EMAIL.into(),
                    );
                };

                let Some(message) = self.current_message.as_mut() else {
                    return SessionReply::ReplyAndContinue(503, Self::RESPONSE_MAIL_FIRST.into());
                };

                message.recipients.push(to_address);

                let response_message = Self::RESPONSE_TO_OK.replace("[email]", &to.address);
                SessionReply::ReplyAndContinue(250, response_message)
            }
            Request::Bdat {
                chunk_size: _,
                is_last: _,
            } => SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into()),
            Request::Noop { value: _ } => {
                // RFC5321, 4.1.1.9
                SessionReply::ReplyAndContinue(250, Self::RESPONSE_OK.into())
            }
            Request::StartTls => {
                SessionReply::ReplyAndContinue(504, Self::RESPONSE_ALREADY_TLS.into())
            }
            Request::Data => {
                // RFC5231, 4.1.1.4
                let Some(NewMessage { recipients, .. }) = self.current_message.as_ref() else {
                    return SessionReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
                };

                if recipients.is_empty() {
                    return SessionReply::ReplyAndContinue(
                        554,
                        Self::RESPONSE_NOVALID_RECIPIENTS.into(),
                    );
                }

                SessionReply::IngestData(354, Self::RESPONSE_START_DATA.into())
            }
            Request::Rset => {
                // RFC5321, 4.1.1.5. Comments about this:
                // - this does not need to clear AUTH status
                // - this does not clear the EHLO status
                self.current_message = None;
                SessionReply::ReplyAndContinue(250, Self::RESPONSE_OK.into())
            }
            Request::Vrfy { value: _ } => {
                // RFC5321, 4.1.1.6
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_NO_VRFY.into())
            }
            Request::Expn { value: _ } => {
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into())
            }
            Request::Help { value: _ } => {
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into())
            }
            Request::Etrn { .. } | Request::Atrn { .. } | Request::Burl { .. } => {
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into())
            }
        }
    }

    fn decode_plain_auth(data: &mut [u8]) -> Result<AttemptedAuth, AttemptedAuthError> {
        // we may need to trim off a trailing CR/LF
        let ascii_len = data.trim_ascii_end().len();
        let data = &mut data[..ascii_len];

        let Ok(decoded) = base64ct::Base64::decode_in_place(data) else {
            return Err(AttemptedAuthError::SyntaxError);
        };

        let mut parts = decoded.split(|&b| b == 0);

        let Some(authcid) = parts.next() else {
            return Err(AttemptedAuthError::SyntaxError);
        };
        if authcid != b"" {
            trace!(
                "Ignoring received authentication identity (authcid): {}",
                String::from_utf8_lossy(authcid)
            );
        }
        let username = parts.next().ok_or(AttemptedAuthError::SyntaxError)?;
        let password = parts.next().ok_or(AttemptedAuthError::SyntaxError)?;
        if parts.count() != 0 {
            return Err(AttemptedAuthError::SyntaxError);
        }

        let username = std::str::from_utf8(username).map_err(|_| AttemptedAuthError::Utf8Error)?;
        let password = std::str::from_utf8(password).map_err(|_| AttemptedAuthError::Utf8Error)?;

        Ok(AttemptedAuth { username, password })
    }

    pub(super) async fn handle_plain_auth(&mut self, data: &mut [u8]) -> (u16, String) {
        let Ok(AttemptedAuth { username, password }) = Self::decode_plain_auth(data) else {
            return (501, Self::RESPONSE_SYNTAX_ERROR.into());
        };

        trace!(
            "decoded credentials, username: {username} password ({} characters)",
            password.len()
        );

        match self.smtp_credentials.find_by_username(username).await {
            Ok(Some(credential)) if credential.verify_password(password) => {
                self.authenticated_credential = Some(credential);

                (235, Self::RESPONSE_AUTH_SUCCCESS.into())
            }
            _ => (535, Self::RESPONSE_AUTH_ERROR.into()),
        }
    }

    pub async fn handle_data(&mut self, data: &[u8]) -> DataReply {
        let Some(NewMessage {
            raw_data: buffer, ..
        }) = self.current_message.as_mut()
        else {
            return DataReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
        };

        buffer.extend_from_slice(data);

        if buffer.len() > Self::MAX_BODY_SIZE as usize {
            debug!("failed to read message: message too big");

            return DataReply::ReplyAndContinue(554, Self::RESPONSE_MESSAGE_REJECTED.into());
        }

        const DATA_END: &[u8] = b"\r\n.\r\n";

        if buffer.ends_with(DATA_END) || buffer == &DATA_END[2..] {
            buffer.truncate(buffer.len() - DATA_END.len());

            let Some(message) = self.current_message.take() else {
                return DataReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
            };

            trace!("received message ({} bytes)", message.raw_data.len());

            if let Err(e) = self.queue.send(message).await {
                debug!("failed to queue message: {e}");

                return DataReply::ReplyAndContinue(554, Self::RESPONSE_MESSAGE_REJECTED.into());
            }

            return DataReply::ReplyAndContinue(250, Self::RESPONSE_MESSAGE_ACCEPTED.into());
        }

        DataReply::ContinueIngest
    }
}
