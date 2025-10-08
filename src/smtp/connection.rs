use smtp_proto::Request;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    time::{Duration, timeout},
};
use tracing::{debug, info, trace};

use crate::{
    messaging::BusClient,
    models::{MessageRepository, SmtpCredentialRepository},
    smtp::session::{DataReply, SessionReply, SmtpResponse, SmtpSession},
};

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("failed to accept connection: {0}")]
    Accept(std::io::Error),
    #[error("failed to write tcp stream: {0}")]
    Write(std::io::Error),
    #[error("failed to read tcp stream: {0}")]
    Read(std::io::Error),
    #[error("connection dropped unexpectedly")]
    Dropped,
    #[error("connection timed out")]
    Timeout(tokio::time::error::Elapsed),
}

const BUFFER_SIZE: usize = 1024;
const CODE_READY: u16 = 220;

pub async fn handle(
    stream: &mut (impl AsyncReadExt + AsyncWriteExt + Unpin),
    server_name: String,
    peer_addr: SocketAddr,
    bus_client: BusClient,
    user_repository: SmtpCredentialRepository,
    message_repository: MessageRepository,
    max_automatic_retries: i32,
) -> Result<(), ConnectionError> {
    let (source, mut sink) = tokio::io::split(stream);

    // NOTE: we re-use this Vec<u8> to avoid re-allocating buffer
    let mut buffer = Vec::with_capacity(BUFFER_SIZE);
    let mut session = SmtpSession::new(
        peer_addr,
        bus_client,
        user_repository,
        message_repository,
        max_automatic_retries,
    );

    let mut reader = BufReader::new(source);

    trace!("handling connection with {}", &session.peer());

    write_reply((CODE_READY, server_name).into(), &mut sink).await?;

    'session: loop {
        read_line(&mut reader, &mut buffer).await?;

        let request = Request::parse(&mut buffer.iter());

        trace!("received request: {:?}", request);

        match session.handle(request).await {
            SessionReply::ReplyAndContinue(response) => {
                write_reply(response, &mut sink).await?;
                continue;
            }
            SessionReply::ReplyAndStop(response) => {
                write_reply(response, &mut sink).await?;
                break;
            }
            SessionReply::RawReply(buf) => {
                sink.write(&buf).await.map_err(ConnectionError::Write)?;
                continue;
            }
            SessionReply::IngestData(response) => {
                write_reply(response, &mut sink).await?;

                'data: loop {
                    read_buf(&mut reader, &mut buffer).await?;

                    match session.handle_data(&buffer).await {
                        DataReply::ContinueIngest => continue 'data,
                        DataReply::ReplyAndContinue(response) => {
                            write_reply(response, &mut sink).await?;
                            continue 'session;
                        }
                    }
                }
            }
            SessionReply::IngestAuth(response) => {
                write_reply(response, &mut sink).await?;
                read_line(&mut reader, &mut buffer).await?;

                let response = session.handle_plain_auth(&mut buffer).await;
                write_reply(response, &mut sink).await?;
            }
        }
    }

    info!("connection handled");

    Ok(())
}

async fn read_buf(
    reader: impl AsyncBufReadExt + Unpin,
    buffer: &mut Vec<u8>,
) -> Result<usize, ConnectionError> {
    buffer.clear();

    timeout(
        Duration::from_secs(300),
        reader.take(BUFFER_SIZE as u64).read_buf(buffer),
    )
    .await
    .map_err(ConnectionError::Timeout)?
    .map_err(ConnectionError::Read)
    .and_then(|size| {
        if size > 0 {
            Ok(size)
        } else {
            Err(ConnectionError::Dropped)
        }
    })
}

async fn read_line(
    reader: impl AsyncBufReadExt + Unpin,
    buffer: &mut Vec<u8>,
) -> Result<usize, ConnectionError> {
    buffer.clear();

    timeout(
        Duration::from_secs(300),
        reader.take(BUFFER_SIZE as u64).read_until(b'\n', buffer),
    )
    .await
    .map_err(ConnectionError::Timeout)?
    .map_err(ConnectionError::Read)
    .and_then(|size| {
        if size > 0 {
            Ok(size)
        } else {
            Err(ConnectionError::Dropped)
        }
    })
}

async fn write_reply(
    response: SmtpResponse,
    mut sink: impl AsyncWriteExt + Unpin,
) -> Result<(), ConnectionError> {
    let n = sink
        .write(format!("{response}\r\n").as_bytes())
        .await
        .map_err(ConnectionError::Write)?;

    if n < 256 {
        debug!("sent: {}", response);
    } else {
        trace!("sent {} bytes", n);
    }

    Ok(())
}
