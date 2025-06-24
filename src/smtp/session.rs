use base64ct::Encoding;
use email_address::EmailAddress;
use smtp_proto::{
    AUTH_PLAIN, EXT_8BIT_MIME, EXT_AUTH, EXT_ENHANCED_STATUS_CODES, EXT_SMTP_UTF8, EhloResponse,
    Request,
};
use std::{fmt::Display, net::SocketAddr};
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

pub struct SmtpResponse(u16, String);

impl Display for SmtpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.0, self.1)
    }
}

type ConstResponse = (u16, &'static str);

impl From<ConstResponse> for SmtpResponse {
    fn from(response: ConstResponse) -> Self {
        Self(response.0, response.1.into())
    }
}

impl From<(u16, String)> for SmtpResponse {
    fn from(response: (u16, String)) -> Self {
        Self(response.0, response.1)
    }
}

impl SmtpResponse {
    fn from_ok(email: String) -> Self {
        SmtpResponse(250, format!("2.1.0 Originator <{email}> ok"))
    }

    fn to_ok(email: String) -> Self {
        SmtpResponse(250, format!("2.1.5 Recipient <{email}> ok"))
    }

    const OK: ConstResponse = (250, "2.0.0 Ok");
    const SYNTAX_ERROR: ConstResponse = (501, "5.5.2 Syntax error");
    const AUTH_SUCCESS: ConstResponse = (235, "2.7.0 Authentication succeeded.");
    const START_DATA: ConstResponse = (354, "3.5.4 Start mail input; end with <CRLF>.<CRLF>");
    const BYE: ConstResponse = (221, "2.0.0 Goodbye");
    const MESSAGE_ACCEPTED: ConstResponse = (250, "2.6.0 Message queued for delivery");
    const MESSAGE_REJECTED: ConstResponse = (554, "5.6.0 Message rejected");
    const BAD_SEQUENCE: ConstResponse = (503, "5.5.1 Bad sequence of commands");
    const MAIL_FIRST: ConstResponse = (503, "5.5.1 Use MAIL first");
    const HELLO_FIRST: ConstResponse = (503, "5.5.1 Be nice and say EHLO first");
    const NOVALID_RECIPIENTS: ConstResponse = (554, "5.5.1 No valid recipients");
    const INVALID_SENDER: ConstResponse = (553, "5.1.7 This sender address is not valid");
    const INVALID_EMAIL: ConstResponse = (553, "5.1.3 This email address is not valid");
    const NESTED_MAIL: ConstResponse = (503, "5.5.1 Error: nested MAIL command");
    const ALREADY_AUTHENTICATED: ConstResponse = (503, "5.5.1 Already authenticated");
    const AUTH_ERROR: ConstResponse = (535, "5.7.8 Authentication credentials invalid");
    const AUTHENTICATION_REQUIRED: ConstResponse = (530, "5.7.1 Authentication required");
    const ALREADY_TLS: ConstResponse = (504, "5.7.4 Already in TLS mode");
    const COMMAND_NOT_IMPLEMENTED: ConstResponse = (502, "5.5.1 Command not implemented");
    const MUST_USE_ESMTP: ConstResponse = (502, "5.5.1 Must use EHLO");
    const NO_VRFY: ConstResponse = (502, "5.5.1 VRFY command is disabled");
    const INGEST_AUTH: ConstResponse = (334, "Tell me your secret.");
    const RATE_LIMIT: ConstResponse = (450, "4.3.2 Sent too many messages, try again later");
}

pub enum SessionReply {
    ReplyAndContinue(SmtpResponse),
    ReplyAndStop(SmtpResponse),
    RawReply(Vec<u8>),
    IngestData(SmtpResponse),
    IngestAuth(SmtpResponse),
}

pub enum DataReply {
    ReplyAndContinue(SmtpResponse),
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

    pub fn new(
        peer_addr: SocketAddr,
        queue: Sender<NewMessage>,
        smtp_credentials: SmtpCredentialRepository,
    ) -> Self {
        Self {
            queue,
            smtp_credentials,
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
                return SessionReply::ReplyAndContinue(SmtpResponse(554, e.to_string()));
            }
        };

        if let Request::Auth { mechanism, .. } = request {
            // This is a workaround as we are not in control of the `Debug` implementation of `Request`.
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
                SessionReply::ReplyAndContinue(SmtpResponse::COMMAND_NOT_IMPLEMENTED.into())
            }
            Request::Helo { host: _ } => {
                SessionReply::ReplyAndContinue(SmtpResponse::MUST_USE_ESMTP.into())
            }
            Request::Auth {
                mechanism,
                initial_response,
            } => {
                // RFC 4954
                if self.authenticated_credential.is_some() {
                    return SessionReply::ReplyAndContinue(
                        SmtpResponse::ALREADY_AUTHENTICATED.into(),
                    );
                }

                if mechanism != AUTH_PLAIN {
                    debug!("Received unsupported AUTH request");
                    return SessionReply::ReplyAndContinue(SmtpResponse::AUTH_ERROR.into());
                }

                debug!("Received AUTH PLAIN");

                if initial_response.is_empty() {
                    return SessionReply::IngestAuth(SmtpResponse::INGEST_AUTH.into());
                }

                let (response, stop) = self
                    .handle_plain_auth(&mut initial_response.into_bytes())
                    .await;

                if stop {
                    SessionReply::ReplyAndStop(response)
                } else {
                    SessionReply::ReplyAndContinue(response)
                }
            }
            Request::Quit => {
                // RFC5321, 4.1.1.10
                SessionReply::ReplyAndStop(SmtpResponse::BYE.into())
            }
            // if the client did not say EHLO, we want to ask for that first instead of processing any of the below commands
            _ignored_command if self.peer_name.is_none() => {
                SessionReply::ReplyAndContinue(SmtpResponse::HELLO_FIRST.into())
            }
            Request::Mail { from } => {
                // RFC5231, 4.1.1.2
                debug!("received MAIL FROM: {}", from.address);

                let Ok(from_address) = from.address.parse::<EmailAddress>() else {
                    return SessionReply::ReplyAndContinue(SmtpResponse::INVALID_SENDER.into());
                };

                let Some(credential) = self.authenticated_credential.as_ref() else {
                    return SessionReply::ReplyAndContinue(
                        SmtpResponse::AUTHENTICATION_REQUIRED.into(),
                    );
                };

                if self.current_message.is_some() {
                    return SessionReply::ReplyAndContinue(SmtpResponse::NESTED_MAIL.into());
                }

                self.current_message = Some(NewMessage::new(credential.id(), from_address));

                SessionReply::ReplyAndContinue(SmtpResponse::from_ok(from.address))
            }
            Request::Rcpt { to } => {
                // RFC5231, 4.1.1.3
                debug!("received RCPT TO: {}", to.address);

                let Ok(to_address) = to.address.parse::<EmailAddress>() else {
                    return SessionReply::ReplyAndContinue(SmtpResponse::INVALID_EMAIL.into());
                };

                let Some(message) = self.current_message.as_mut() else {
                    return SessionReply::ReplyAndContinue(SmtpResponse::MAIL_FIRST.into());
                };

                message.recipients.push(to_address);

                SessionReply::ReplyAndContinue(SmtpResponse::to_ok(to.address))
            }
            Request::Bdat {
                chunk_size: _,
                is_last: _,
            } => SessionReply::ReplyAndContinue(SmtpResponse::COMMAND_NOT_IMPLEMENTED.into()),
            Request::Noop { value: _ } => {
                // RFC5321, 4.1.1.9
                SessionReply::ReplyAndContinue(SmtpResponse::OK.into())
            }
            Request::StartTls => SessionReply::ReplyAndContinue(SmtpResponse::ALREADY_TLS.into()),
            Request::Data => {
                // RFC5231, 4.1.1.4
                let Some(NewMessage { recipients, .. }) = self.current_message.as_ref() else {
                    return SessionReply::ReplyAndContinue(SmtpResponse::BAD_SEQUENCE.into());
                };

                if recipients.is_empty() {
                    return SessionReply::ReplyAndContinue(SmtpResponse::NOVALID_RECIPIENTS.into());
                }

                SessionReply::IngestData(SmtpResponse::START_DATA.into())
            }
            Request::Rset => {
                // RFC5321, 4.1.1.5. Comments about this:
                // - this does not need to clear AUTH status
                // - this does not clear the EHLO status
                self.current_message = None;
                SessionReply::ReplyAndContinue(SmtpResponse::OK.into())
            }
            Request::Vrfy { value: _ } => {
                // RFC5321, 4.1.1.6
                SessionReply::ReplyAndContinue(SmtpResponse::NO_VRFY.into())
            }
            Request::Expn { value: _ } => {
                SessionReply::ReplyAndContinue(SmtpResponse::COMMAND_NOT_IMPLEMENTED.into())
            }
            Request::Help { value: _ } => {
                SessionReply::ReplyAndContinue(SmtpResponse::COMMAND_NOT_IMPLEMENTED.into())
            }
            Request::Etrn { .. } | Request::Atrn { .. } | Request::Burl { .. } => {
                SessionReply::ReplyAndContinue(SmtpResponse::COMMAND_NOT_IMPLEMENTED.into())
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

        if let Some(authcid) = parts.next() {
            trace!(
                "Ignoring received authentication identity (authcid): {}",
                String::from_utf8_lossy(authcid)
            );
        } else {
            return Err(AttemptedAuthError::SyntaxError);
        };

        let username = parts.next().ok_or(AttemptedAuthError::SyntaxError)?;
        let password = parts.next().ok_or(AttemptedAuthError::SyntaxError)?;
        if parts.count() != 0 {
            return Err(AttemptedAuthError::SyntaxError);
        }

        let username = std::str::from_utf8(username).map_err(|_| AttemptedAuthError::Utf8Error)?;
        let password = std::str::from_utf8(password).map_err(|_| AttemptedAuthError::Utf8Error)?;

        Ok(AttemptedAuth { username, password })
    }

    /// Returns a response and whether or not the session should stop
    pub(super) async fn handle_plain_auth(&mut self, data: &mut [u8]) -> (SmtpResponse, bool) {
        let Ok(AttemptedAuth { username, password }) = Self::decode_plain_auth(data) else {
            return (SmtpResponse::SYNTAX_ERROR.into(), false);
        };

        trace!(
            "decoded credentials, username: {username} password ({} characters)",
            password.len()
        );

        let Ok(Some((credential, ratelimit))) = self
            .smtp_credentials
            .find_by_username_rate_limited(username)
            .await
        else {
            return (SmtpResponse::AUTH_ERROR.into(), false);
        };

        if !credential.verify_password(password) {
            return (SmtpResponse::AUTH_ERROR.into(), false);
        }

        if ratelimit <= 0 {
            return (SmtpResponse::RATE_LIMIT.into(), true); // rate limited, stop session
        }

        self.authenticated_credential = Some(credential);
        (SmtpResponse::AUTH_SUCCESS.into(), false)
    }

    pub async fn handle_data(&mut self, data: &[u8]) -> DataReply {
        let Some(NewMessage {
            raw_data: buffer, ..
        }) = self.current_message.as_mut()
        else {
            return DataReply::ReplyAndContinue(SmtpResponse::BAD_SEQUENCE.into());
        };

        buffer.extend_from_slice(data);

        if buffer.len() > Self::MAX_BODY_SIZE as usize {
            debug!("failed to read message: message too big");

            return DataReply::ReplyAndContinue(SmtpResponse::MESSAGE_REJECTED.into());
        }

        const DATA_END: &[u8] = b"\r\n.\r\n";

        if buffer.ends_with(DATA_END) || buffer == &DATA_END[2..] {
            buffer.truncate(buffer.len() - DATA_END.len());

            let Some(message) = self.current_message.take() else {
                return DataReply::ReplyAndContinue(SmtpResponse::BAD_SEQUENCE.into());
            };

            trace!("received message ({} bytes)", message.raw_data.len());

            if let Err(e) = self.queue.send(message).await {
                debug!("failed to queue message: {e}");

                return DataReply::ReplyAndContinue(SmtpResponse::MESSAGE_REJECTED.into());
            }

            return DataReply::ReplyAndContinue(SmtpResponse::MESSAGE_ACCEPTED.into());
        }

        DataReply::ContinueIngest
    }
}
