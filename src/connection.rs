use mail_parser::decoders::base64::base64_decode;
use smtp_proto::*;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc::Sender,
};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tracing::{debug, trace};

use crate::{
    message::Message,
    users::{User, UserRepository},
};

#[derive(Debug, Error)]
pub(crate) enum ConnectionError {
    #[error("failed to accept connection: {0}")]
    Accept(std::io::Error),
    #[error("failed to write tcp stream: {0}")]
    Write(std::io::Error),
    #[error("failed to read tcp stream: {0}")]
    Read(std::io::Error),
    #[error("failed to base 64 decode user credentials")]
    DecodeCredentials,
    #[error("message body too large")]
    MessageTooBig,
    #[error("received message is empty")]
    EmptyMessage,
    #[error("failed to send message to queue: {0}")]
    WriteQueue(tokio::sync::mpsc::error::SendError<Message>),
}

#[derive(Debug, PartialEq)]
enum ConnectionState {
    Accepting,
    Ready,
    Authenticated,
    FromReceived,
    RecipientsReceived,
}

pub(crate) struct Connection {
    acceptor: TlsAcceptor,
    stream: TcpStream,
    peer_addr: SocketAddr,
    authenticated_user: Option<User>,
    buffer: Vec<u8>,
    current_message: Option<Message>,
    connectio_state: ConnectionState,
}

impl Connection {
    const BUFFER_SIZE: usize = 1024;
    const MAX_BODY_SIZE: u64 = 20 * 1024 * 1024;
    const SERVER_NAME: &str = "localhost";
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

    pub(crate) fn new(acceptor: TlsAcceptor, stream: TcpStream, peer_addr: SocketAddr) -> Self {
        Self {
            acceptor,
            stream,
            peer_addr,
            buffer: Vec::with_capacity(Self::BUFFER_SIZE),
            // message
            current_message: None,
            authenticated_user: None,
            connectio_state: ConnectionState::Accepting,
        }
    }

    pub(crate) async fn handle(
        mut self,
        queue: Sender<Message>,
        user_repository: UserRepository,
    ) -> Result<(), ConnectionError> {
        let tls_stream: TlsStream<TcpStream> = self
            .acceptor
            .accept(self.stream)
            .await
            .map_err(ConnectionError::Accept)?;

        trace!("secure connection with {}", &self.peer_addr);

        let (stream, mut sink) = tokio::io::split(tls_stream);
        let mut reader = BufReader::new(stream);

        Connection::reply(220, Self::SERVER_NAME, &mut sink).await?;

        loop {
            Connection::read_line(&mut reader, &mut self.buffer).await?;

            let Ok(request) = Request::parse(&mut self.buffer.iter()) else {
                Connection::reply(500, Self::RESPONSE_SYNTAX_ERROR, &mut sink).await?;
                debug!("failed to parse request");
                continue;
            };

            trace!("received request: {:?}", request);

            match request {
                Request::Ehlo { host } => {
                    if self.connectio_state != ConnectionState::Accepting {
                        Connection::reply(503, Self::RESPONSE_BAD_SEQUENCE, &mut sink).await?;
                        continue;
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
                    let n = sink.write(&buf).await.map_err(ConnectionError::Write)?;

                    self.connectio_state = ConnectionState::Ready;

                    trace!("sent {} bytes", n);
                }
                Request::Lhlo { host: _ } => todo!(),
                Request::Helo { host: _ } => todo!(),
                Request::Mail { from } => {
                    debug!("received MAIL FROM: {}", from.address);

                    if self.connectio_state != ConnectionState::Authenticated {
                        Connection::reply(530, Self::RESPONSE_AUTHENTICATION_REQUIRED, &mut sink)
                            .await?;
                        continue;
                    }

                    let Some(user) = self.authenticated_user.as_ref() else {
                        Connection::reply(530, Self::RESPONSE_AUTHENTICATION_REQUIRED, &mut sink)
                            .await?;
                        continue;
                    };

                    self.current_message = Some(Message::new(user.get_id(), from.address.clone()));
                    self.connectio_state = ConnectionState::FromReceived;

                    let response_message = Self::RESPONSE_FROM_OK.replace("[email]", &from.address);
                    Connection::reply(250, &response_message, &mut sink).await?;
                }
                Request::Rcpt { to } => {
                    debug!("received RCPT TO: {}", to.address);

                    if self.connectio_state != ConnectionState::FromReceived
                        && self.connectio_state != ConnectionState::RecipientsReceived
                    {
                        Connection::reply(503, Self::RESPONSE_BAD_SEQUENCE, &mut sink).await?;
                        continue;
                    }

                    let Some(message) = self.current_message.as_mut() else {
                        Connection::reply(503, Self::RESPONSE_BAD_SEQUENCE, &mut sink).await?;
                        continue;
                    };

                    message.add_recipient(to.address.clone());

                    self.connectio_state = ConnectionState::RecipientsReceived;

                    let response_message = Self::RESPONSE_TO_OK.replace("[email]", &to.address);
                    Connection::reply(250, &response_message, &mut sink).await?
                }
                Request::Bdat {
                    chunk_size: _,
                    is_last: _,
                } => todo!(),
                Request::Auth {
                    mechanism,
                    initial_response,
                } => {
                    if self.connectio_state != ConnectionState::Ready {
                        Connection::reply(503, Self::RESPONSE_BAD_SEQUENCE, &mut sink).await?;
                        continue;
                    }

                    if mechanism == AUTH_PLAIN {
                        debug!("Received AUTH PLAIN");

                        if initial_response.is_empty() {
                            Connection::reply(334, "Go ahead.", &mut sink).await?;
                            continue;
                        }

                        let decoded = base64_decode(initial_response.as_bytes())
                            .ok_or(ConnectionError::DecodeCredentials)?;

                        let parts = decoded.split(|&b| b == 0).collect::<Vec<_>>();

                        if parts.len() != 3 {
                            Connection::reply(501, Self::RESPONSE_SYNTAX_ERROR, &mut sink).await?;
                            continue;
                        }

                        let username = String::from_utf8_lossy(parts[1]);
                        let password = String::from_utf8_lossy(parts[2]);

                        trace!(
                            "decoded credentials, username: {username} password ({} characters)",
                            password.len()
                        );

                        let Ok(Some(user)) = user_repository.find_by_username(&username).await
                        else {
                            Connection::reply(535, Self::RESPONSE_AUTH_ERROR, &mut sink).await?;
                            continue;
                        };

                        if !user.verify_password(&password) {
                            Connection::reply(535, Self::RESPONSE_AUTH_ERROR, &mut sink).await?;
                            continue;
                        }

                        self.authenticated_user = Some(user);
                        self.connectio_state = ConnectionState::Authenticated;

                        Connection::reply(235, Self::RESPONSE_AUTH_SUCCCESS, &mut sink).await?;
                    }
                }
                Request::Noop { value: _ } => {
                    Connection::reply(250, Self::RESPONSE_OK, &mut sink).await?;
                }
                Request::StartTls => {
                    Connection::reply(504, Self::RESPONSE_ALREADY_TLS, &mut sink).await?;
                }
                Request::Data => {
                    if self.connectio_state != ConnectionState::RecipientsReceived {
                        Connection::reply(503, Self::RESPONSE_BAD_SEQUENCE, &mut sink).await?;
                        continue;
                    }

                    Connection::reply(354, Self::RESPONSE_START_DATA, &mut sink).await?;

                    let Some(mut message) = self.current_message.take() else {
                        Connection::reply(503, Self::RESPONSE_BAD_SEQUENCE, &mut sink).await?;
                        continue;
                    };

                    let mut raw_data = Vec::new();

                    loop {
                        Connection::read_buf(&mut reader, &mut self.buffer).await?;

                        raw_data.extend_from_slice(&self.buffer);

                        if raw_data.ends_with(Self::DATA_END) {
                            break;
                        }

                        if raw_data.len() > Connection::MAX_BODY_SIZE as usize {
                            Connection::reply(554, Self::RESPONSE_MESSAGE_REJECTED, &mut sink)
                                .await?;
                            debug!("failed to read message: message too big");

                            return Err(ConnectionError::MessageTooBig);
                        }
                    }

                    if raw_data.is_empty() {
                        Connection::reply(554, Self::RESPONSE_MESSAGE_REJECTED, &mut sink).await?;
                        debug!("failed to read message: empty message");

                        return Err(ConnectionError::EmptyMessage);
                    }

                    trace!(
                        "received message {:?} ({} bytes)",
                        message.get_id(),
                        raw_data.len()
                    );
                    let response_message = Self::RESPONSE_MESSAGE_ACCEPTED
                        .replace("[id]", &message.get_id().to_string());

                    message.set_raw_data(raw_data);

                    queue
                        .send(message)
                        .await
                        .map_err(ConnectionError::WriteQueue)?;

                    self.connectio_state = ConnectionState::Authenticated;

                    Connection::reply(250, &response_message, &mut sink).await?;
                }
                Request::Rset => todo!(),
                Request::Quit => {
                    Connection::reply(221, Self::RESPONSE_BYE, &mut sink).await?;
                    break;
                }
                Request::Vrfy { value: _ } => todo!(),
                Request::Expn { value: _ } => todo!(),
                Request::Help { value: _ } => todo!(),
                Request::Etrn { .. } | Request::Atrn { .. } | Request::Burl { .. } => {
                    Connection::reply(502, Self::RESPONSE_COMMAND_NOT_IMPLEMENTED, &mut sink)
                        .await?;
                }
            }
        }

        // send tls close notify
        sink.shutdown().await.map_err(ConnectionError::Write)?;

        Ok(())
    }

    async fn read_buf(
        reader: impl AsyncBufReadExt + Unpin,
        buffer: &mut Vec<u8>,
    ) -> Result<usize, ConnectionError> {
        buffer.clear();

        reader
            .take(Connection::BUFFER_SIZE as u64)
            .read_buf(buffer)
            .await
            .map_err(ConnectionError::Read)
    }

    async fn read_line(
        reader: impl AsyncBufReadExt + Unpin,
        buffer: &mut Vec<u8>,
    ) -> Result<usize, ConnectionError> {
        buffer.clear();

        reader
            .take(Connection::BUFFER_SIZE as u64)
            .read_until(b'\n', buffer)
            .await
            .map_err(ConnectionError::Read)
    }

    async fn reply(
        code: u16,
        message: &str,
        mut sink: impl AsyncWriteExt + Unpin,
    ) -> Result<(), ConnectionError> {
        let n = sink
            .write(format!("{code} {message}\r\n").as_bytes())
            .await
            .map_err(ConnectionError::Write)?;

        if n < 256 {
            debug!("sent: {} {}", code, message);
        } else {
            trace!("sent {} bytes", n);
        }

        Ok(())
    }
}
