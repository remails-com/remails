use anyhow::Context;
use mail_parser::decoders::base64::base64_decode;
use smtp_proto::*;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream, sync::mpsc::Sender,
};
use tokio_rustls::{server::TlsStream, TlsAcceptor};
use tracing::{debug, trace};

use crate::message::Message;

pub(crate) struct Connection {
    acceptor: TlsAcceptor,
    stream: TcpStream,
    peer_addr: SocketAddr,
    buffer: Vec<u8>,
    current_message: Option<Message>,
}

impl Connection {
    const BUFFER_SIZE: usize = 1024;
    const MAX_BODY_SIZE: u64 = 20 * 1024 * 1024;
    const SERVER_NAME: &str = "localhost";
    const DATA_END: &[u8] = b"\r\n.\r\n";

    const RESPONSE_FROM_OK: &str = "2.1.0 Originator <[email]> ok";
    const RESPONSE_TO_OK: &str = "2.1.5 Recipient <[email]> ok";
    const RESPONSE_SYNTAX_ERROR: &str = "5.5.2 Syntax error";
    const RESPONSE_AUTH_SUCCCESS: &str = "2.7.0 Authentication succeeded.";
    const RESPONSE_START_DATA: &str = "3.5.4 Start mail input; end with <CRLF>.<CRLF>";
    const RESPONSE_BYE: &str = "2.0.0 Goodbye";
    const RESPONSE_MESSAGE_ACCEPTED: &str =
        "2.6.0 Message accepted for delivery, queued as [id]";
    const RESPONSE_MESSAGE_REJECTED: &str = "5.6.0 Message rejected";
    const RESPONSE_BAD_SEQUENCE: &str = "5.5.1 Bad sequence of commands";

    pub(crate) fn new(acceptor: TlsAcceptor, stream: TcpStream, peer_addr: SocketAddr) -> Self {
        Self {
            acceptor,
            stream,
            peer_addr,
            buffer: Vec::with_capacity(Self::BUFFER_SIZE),
            // message
            current_message: None,
        }
    }

    pub(crate) async fn handle(mut self, queue: Sender<Message>) -> anyhow::Result<()> {
        let tls_stream: TlsStream<TcpStream> = self.acceptor.accept(self.stream).await?;

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
                    let mut response = EhloResponse::new(host);
                    response.capabilities = EXT_ENHANCED_STATUS_CODES
                        | EXT_8BIT_MIME
                        | EXT_BINARY_MIME
                        | EXT_SMTP_UTF8
                        | EXT_AUTH;

                    response.auth_mechanisms = AUTH_PLAIN | AUTH_LOGIN;

                    let mut buf = Vec::with_capacity(64);
                    response.write(&mut buf).ok();
                    let n = sink.write(&buf).await?;

                    trace!("sent {} bytes", n);
                }
                Request::Lhlo { host: _ } => todo!(),
                Request::Helo { host: _ } => todo!(),
                Request::Mail { from } => {
                    debug!("received MAIL FROM: {}", from.address);

                    self.current_message = Some(Message::new(from.address.clone()));

                    let response_message = Self::RESPONSE_FROM_OK.replace("[email]", &from.address);
                    Connection::reply(250, &response_message, &mut sink).await?;
                }
                Request::Rcpt { to } => {
                    debug!("received RCPT TO: {}", to.address);

                    let Some(message) = self.current_message.as_mut() else {
                        Connection::reply(503, Self::RESPONSE_BAD_SEQUENCE, &mut sink).await?;
                        continue;
                    };

                    message.add_recipient(to.address.clone());

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
                    if mechanism == AUTH_PLAIN {
                        debug!("Received AUTH PLAIN");

                        if initial_response.is_empty() {
                            Connection::reply(334, "Go ahead.", &mut sink).await?;
                            continue;
                        }

                        let decoded =
                            base64_decode(initial_response.as_bytes()).context("Invalid base64")?;

                        let parts = decoded.split(|&b| b == 0).collect::<Vec<_>>();

                        trace!(
                            "Decoded: {} {}",
                            String::from_utf8_lossy(parts[1]),
                            String::from_utf8_lossy(parts[2])
                        );

                        // TODO authenticate

                        Connection::reply(235, Self::RESPONSE_AUTH_SUCCCESS, &mut sink).await?;
                    }
                }
                Request::Noop { value: _ } => todo!(),
                Request::Vrfy { value: _ } => todo!(),
                Request::Expn { value: _ } => todo!(),
                Request::Help { value: _ } => todo!(),
                Request::Etrn { name: _ } => todo!(),
                Request::Atrn { domains: _ } => todo!(),
                Request::Burl { uri: _, is_last: _ } => todo!(),
                Request::StartTls => todo!(),
                Request::Data => {
                    Connection::reply(354, Self::RESPONSE_START_DATA, &mut sink).await?;

                    let Some(mut message) = self.current_message.take() else {
                        Connection::reply(503, Self::RESPONSE_BAD_SEQUENCE, &mut sink).await?;
                        continue;
                    };

                    let mut raw_message = Vec::new();

                    loop {
                        Connection::read_buf(&mut reader, &mut self.buffer).await?;

                        raw_message.extend_from_slice(&self.buffer);

                        if raw_message.ends_with(Self::DATA_END) {
                            break;
                        }

                        if raw_message.len() > Connection::MAX_BODY_SIZE as usize {
                            Connection::reply(554, Self::RESPONSE_MESSAGE_REJECTED, &mut sink)
                                .await?;
                            debug!("failed to read message: message too big");

                            return Err(anyhow::anyhow!("message too big"));
                        }
                    }

                    if raw_message.len() == 0 {
                        Connection::reply(554, Self::RESPONSE_MESSAGE_REJECTED, &mut sink).await?;
                        debug!("failed to read message: empty message");

                        return Err(anyhow::anyhow!("empty message"));
                    }

                    trace!("received message {:?} ({} bytes)", message.get_id(), raw_message.len());
                    let response_message =
                        Self::RESPONSE_MESSAGE_ACCEPTED.replace("[id]", &message.get_id().to_string());

                    message.set_raw_message(raw_message);

                    queue.send(message).await?;

                    Connection::reply(250, &response_message, &mut sink).await?;
                }
                Request::Rset => todo!(),
                Request::Quit => {
                    Connection::reply(221, Self::RESPONSE_BYE, &mut sink).await?;
                    break;
                }
            }
        }

        // send tls close notify
        sink.shutdown().await?;

        Ok(())
    }

    async fn read_buf(
        reader: impl AsyncBufReadExt + Unpin,
        buffer: &mut Vec<u8>,
    ) -> anyhow::Result<usize> {
        buffer.clear();

        reader
            .take(Connection::BUFFER_SIZE as u64)
            .read_buf(buffer)
            .await
            .context("failed to read message")
    }

    async fn read_line(
        reader: impl AsyncBufReadExt + Unpin,
        buffer: &mut Vec<u8>,
    ) -> anyhow::Result<()> {
        buffer.clear();

        reader
            .take(Connection::BUFFER_SIZE as u64)
            .read_until(b'\n', buffer)
            .await
            .context("failed to read command")?;

        Ok(())
    }

    async fn reply(
        code: u16,
        message: &str,
        mut sink: impl AsyncWriteExt + Unpin,
    ) -> anyhow::Result<()> {
        let n = sink
            .write(format!("{code} {message}\r\n").as_bytes())
            .await?;

        if n < 256 {
            debug!("sent: {} {}", code, message);
        } else {
            trace!("sent {} bytes", n);
        }

        Ok(())
    }
}
