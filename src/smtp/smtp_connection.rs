use smtp_proto::*;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc::Sender,
};
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use tracing::{debug, info, trace};

use crate::{message::Message, smtp::smtp_session::SessionReply, user::UserRepository};

use super::smtp_session::SmtpSession;

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("failed to accept connection: {0}")]
    Accept(std::io::Error),
    #[error("failed to write tcp stream: {0}")]
    Write(std::io::Error),
    #[error("failed to read tcp stream: {0}")]
    Read(std::io::Error),
}

pub struct SmtpConnection {
    acceptor: TlsAcceptor,
    stream: TcpStream,
    peer_addr: SocketAddr,
    buffer: Vec<u8>,
    session: SmtpSession,
}

impl SmtpConnection {
    const BUFFER_SIZE: usize = 1024;
    const SERVER_NAME: &str = "localhost";

    pub fn new(
        acceptor: TlsAcceptor,
        stream: TcpStream,
        peer_addr: SocketAddr,
        queue: Sender<Message>,
        user_repository: UserRepository,
    ) -> Self {
        Self {
            acceptor,
            stream,
            peer_addr,
            buffer: Vec::with_capacity(Self::BUFFER_SIZE),
            session: SmtpSession::new(peer_addr, queue, user_repository),
        }
    }

    pub async fn handle(mut self) -> Result<(), ConnectionError> {
        let tls_stream: TlsStream<TcpStream> = self
            .acceptor
            .accept(self.stream)
            .await
            .map_err(ConnectionError::Accept)?;

        trace!("secure connection with {}", &self.peer_addr);

        let (stream, mut sink) = tokio::io::split(tls_stream);
        let mut reader = BufReader::new(stream);

        SmtpConnection::reply(220, Self::SERVER_NAME, &mut sink).await?;

        loop {
            SmtpConnection::read_line(&mut reader, &mut self.buffer).await?;

            let request = Request::parse(&mut self.buffer.iter());

            trace!("received request: {:?}", request);

            let reply = self.session.handle(request).await;

            match reply {
                SessionReply::ReplyAndContinue(code, message) => {
                    SmtpConnection::reply(code, &message, &mut sink).await?;
                    continue;
                }
                SessionReply::ReplyAndStop(code, message) => {
                    SmtpConnection::reply(code, &message, &mut sink).await?;
                    break;
                }
                SessionReply::RawReply(buf) => {
                    sink.write(&buf).await.map_err(ConnectionError::Write)?;
                    continue;
                }
                SessionReply::IngestData(code, message) => {
                    SmtpConnection::reply(code, &message, &mut sink).await?;

                    loop {
                        SmtpConnection::read_buf(&mut reader, &mut self.buffer).await?;
                        let reply = self.session.handle_data(&self.buffer).await;

                        match reply {
                            SessionReply::ContinueIngest => continue,
                            SessionReply::ReplyAndContinue(code, message) => {
                                SmtpConnection::reply(code, &message, &mut sink).await?;
                                break;
                            }
                            _ => break,
                        }
                    }
                }
                SessionReply::ContinueIngest => {}
            }
        }

        // send tls close notify
        sink.shutdown().await.map_err(ConnectionError::Write)?;
        info!("connection handled");

        Ok(())
    }

    async fn read_buf(
        reader: impl AsyncBufReadExt + Unpin,
        buffer: &mut Vec<u8>,
    ) -> Result<usize, ConnectionError> {
        buffer.clear();

        reader
            .take(SmtpConnection::BUFFER_SIZE as u64)
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
            .take(SmtpConnection::BUFFER_SIZE as u64)
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
