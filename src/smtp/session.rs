use mail_parser::decoders::base64::base64_decode;
use smtp_proto::*;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use tracing::{debug, trace};

use crate::models::{Message, SmtpCredential, SmtpCredentialRepository};

#[derive(Debug, PartialEq)]
enum SessionState {
    Accepting,
    Ready,
    Authenticated,
    FromReceived,
    RecipientsReceived,
    IngestingData,
}

pub struct SmtpSession {
    queue: Sender<Message>,
    smtp_credentials: SmtpCredentialRepository,

    peer_addr: SocketAddr,
    state: SessionState,
    authenticated_credential: Option<SmtpCredential>,
    current_message: Option<Message>,
    buffer: Vec<u8>,
}

pub enum SessionReply {
    ReplyAndContinue(u16, String),
    ReplyAndStop(u16, String),
    RawReply(Vec<u8>),
    IngestData(u16, String),
}

pub enum DataReply {
    ReplyAndContinue(u16, String),
    ContinueIngest,
}

impl SmtpSession {
    const MAX_BODY_SIZE: u64 = 20 * 1024 * 1024;
    const DATA_END: &[u8] = b"\r\n.\r\n";

    const RESPONSE_OK: &str = "2.0.0 Ok";
    const RESPONSE_FROM_OK: &str = "2.1.0 Originator <[email]> ok";
    const RESPONSE_TO_OK: &str = "2.1.5 Recipient <[email]> ok";
    const RESPONSE_SYNTAX_ERROR: &str = "5.5.2 Syntax error";
    const RESPONSE_AUTH_SUCCCESS: &str = "2.7.0 Authentication succeeded.";
    const RESPONSE_START_DATA: &str = "3.5.4 Start mail input; end with <CRLF>.<CRLF>";
    const RESPONSE_BYE: &str = "2.0.0 Goodbye";
    const RESPONSE_MESSAGE_ACCEPTED: &str = "2.6.0 Message accepted for delivery, queued as [id]";
    const RESPONSE_MESSAGE_REJECTED: &str = "5.6.0 Message rejected";
    const RESPONSE_BAD_SEQUENCE: &str = "5.5.1 Bad sequence of commands";
    const RESPONSE_AUTH_ERROR: &str = "5.7.8 Authentication credentials invalid";
    const RESPONSE_AUTHENTICATION_REQUIRED: &str = "5.7.1 Authentication required";
    const RESPONSE_ALREADY_TLS: &str = "5.7.4 Already in TLS mode";
    const RESPONSE_COMMAND_NOT_IMPLEMENTED: &str = "5.5.1 Command not implemented";

    pub fn new(
        peer_addr: SocketAddr,
        queue: Sender<Message>,
        user_repository: SmtpCredentialRepository,
    ) -> Self {
        Self {
            queue,
            smtp_credentials: user_repository,
            peer_addr,
            state: SessionState::Accepting,
            current_message: None,
            authenticated_credential: None,
            buffer: Vec::new(),
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

                return SessionReply::ReplyAndContinue(500, e.to_string());
            }
        };

        trace!("received request: {request:?} from {}", self.peer_addr);

        match request {
            Request::Ehlo { host } => {
                if self.state != SessionState::Accepting {
                    return SessionReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
                }

                let mut response = EhloResponse::new(host);
                response.capabilities = EXT_ENHANCED_STATUS_CODES
                    | EXT_8BIT_MIME
                    | EXT_BINARY_MIME
                    | EXT_SMTP_UTF8
                    | EXT_AUTH;

                response.auth_mechanisms = AUTH_PLAIN | AUTH_LOGIN;

                let mut buf = Vec::with_capacity(64);
                response.write(&mut buf).ok();

                self.state = SessionState::Ready;

                SessionReply::RawReply(buf)
            }
            Request::Lhlo { host: _ } => {
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into())
            }
            Request::Helo { host: _ } => {
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into())
            }
            Request::Mail { from } => {
                debug!("received MAIL FROM: {}", from.address);

                if self.state != SessionState::Authenticated {
                    return SessionReply::ReplyAndContinue(
                        530,
                        Self::RESPONSE_AUTHENTICATION_REQUIRED.into(),
                    );
                }

                let Some(credential) = self.authenticated_credential.as_ref() else {
                    return SessionReply::ReplyAndContinue(
                        530,
                        Self::RESPONSE_AUTHENTICATION_REQUIRED.into(),
                    );
                };

                self.current_message = Some(Message::new(credential.id(), from.address.clone()));
                self.state = SessionState::FromReceived;

                let response_message = Self::RESPONSE_FROM_OK.replace("[email]", &from.address);
                SessionReply::ReplyAndContinue(250, response_message)
            }
            Request::Rcpt { to } => {
                debug!("received RCPT TO: {}", to.address);

                if self.state != SessionState::FromReceived
                    && self.state != SessionState::RecipientsReceived
                {
                    return SessionReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
                }

                let Some(message) = self.current_message.as_mut() else {
                    return SessionReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
                };

                message.recipients.push(to.address.clone());

                self.state = SessionState::RecipientsReceived;

                let response_message = Self::RESPONSE_TO_OK.replace("[email]", &to.address);
                SessionReply::ReplyAndContinue(250, response_message)
            }
            Request::Bdat {
                chunk_size: _,
                is_last: _,
            } => SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into()),
            Request::Auth {
                mechanism,
                initial_response,
            } => {
                if self.state != SessionState::Ready {
                    return SessionReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
                }

                if mechanism == AUTH_PLAIN {
                    debug!("Received AUTH PLAIN");

                    if initial_response.is_empty() {
                        return SessionReply::ReplyAndContinue(334, "Go ahead.".into());
                    }

                    let Some(decoded) = base64_decode(initial_response.as_bytes()) else {
                        return SessionReply::ReplyAndContinue(
                            501,
                            Self::RESPONSE_SYNTAX_ERROR.into(),
                        );
                    };

                    let parts = decoded.split(|&b| b == 0).collect::<Vec<_>>();

                    if parts.len() != 3 {
                        return SessionReply::ReplyAndContinue(
                            501,
                            Self::RESPONSE_SYNTAX_ERROR.into(),
                        );
                    }

                    let username = String::from_utf8_lossy(parts[1]);
                    let password = String::from_utf8_lossy(parts[2]);

                    trace!(
                        "decoded credentials, username: {username} password ({} characters)",
                        password.len()
                    );

                    let Ok(Some(credential)) =
                        self.smtp_credentials.find_by_username(&username).await
                    else {
                        return SessionReply::ReplyAndContinue(
                            535,
                            Self::RESPONSE_AUTH_ERROR.into(),
                        );
                    };

                    if !credential.verify_password(&password) {
                        return SessionReply::ReplyAndContinue(
                            535,
                            Self::RESPONSE_AUTH_ERROR.into(),
                        );
                    }

                    self.authenticated_credential = Some(credential);
                    self.state = SessionState::Authenticated;

                    SessionReply::ReplyAndContinue(235, Self::RESPONSE_AUTH_SUCCCESS.into())
                } else {
                    // other authentication methods
                    SessionReply::ReplyAndContinue(535, Self::RESPONSE_AUTH_ERROR.into())
                }
            }
            Request::Noop { value: _ } => {
                SessionReply::ReplyAndContinue(250, Self::RESPONSE_OK.into())
            }
            Request::StartTls => {
                SessionReply::ReplyAndContinue(504, Self::RESPONSE_ALREADY_TLS.into())
            }
            Request::Data => {
                if self.state != SessionState::RecipientsReceived {
                    return SessionReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
                }

                self.state = SessionState::IngestingData;

                SessionReply::IngestData(354, Self::RESPONSE_START_DATA.into())
            }
            Request::Rset => {
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into())
            }
            Request::Quit => SessionReply::ReplyAndStop(221, Self::RESPONSE_BYE.into()),
            Request::Vrfy { value: _ } => {
                SessionReply::ReplyAndContinue(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED.into())
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

    pub async fn handle_data(&mut self, data: &[u8]) -> DataReply {
        if self.state != SessionState::IngestingData {
            return DataReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
        }

        self.buffer.extend_from_slice(data);

        if self.buffer.len() > Self::MAX_BODY_SIZE as usize {
            debug!("failed to read message: message too big");

            return DataReply::ReplyAndContinue(554, Self::RESPONSE_MESSAGE_REJECTED.into());
        }

        if self.buffer.ends_with(Self::DATA_END) {
            let Some(mut message) = self.current_message.take() else {
                return DataReply::ReplyAndContinue(503, Self::RESPONSE_BAD_SEQUENCE.into());
            };

            trace!(
                "received message {:?} ({} bytes)",
                message.id(),
                self.buffer.len()
            );
            let response_message =
                Self::RESPONSE_MESSAGE_ACCEPTED.replace("[id]", &message.id().to_string());

            message.raw_data = Some(std::mem::take(&mut self.buffer));

            if let Err(e) = self.queue.send(message).await {
                debug!("failed to queue message: {e}");

                return DataReply::ReplyAndContinue(554, Self::RESPONSE_MESSAGE_REJECTED.into());
            }

            self.state = SessionState::Authenticated;

            return DataReply::ReplyAndContinue(250, response_message);
        }

        DataReply::ContinueIngest
    }
}
