use crate::smtp::proxy_protocol::Error::InvalidProxyProtocolHeader;
use ppp::{HeaderResult, v2, v2::ParseError};
use std::net::IpAddr;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};

const PROTOCOL_SIGNATURE: [u8; 12] = [
    0x0D, 0x0A, 0x0D, 0x0A, 0x00, 0x0D, 0x0A, 0x51, 0x55, 0x49, 0x54, 0x0A,
];

/// The prefix length of a v2 header in bytes.
const V2_PREFIX_LEN: usize = 12;
/// The minimum length of a v2 header in bytes.
const V2_MINIMUM_LEN: usize = 16;
/// The index of the start of the big-endian u16 length in the v2 header.
const V2_LENGTH_INDEX: usize = 14;

const READ_BUFFER_LEN: usize = 512;

pub struct ConnectionInfo {
    pub source_ip: IpAddr,
    pub destination_ip: IpAddr,
    pub source_port: u16,
    pub destination_port: u16,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid Proxy Protocol header")]
    InvalidProxyProtocolHeader,
    #[error("Unsupported: {0}")]
    Unsupported(&'static str),
    #[error["Parse error: {0}"]]
    Parse(ParseError),
}

pub(super) async fn handle_proxy_protocol<IO>(
    mut stream: IO,
) -> Result<(IO, Option<ConnectionInfo>), Error>
where
    IO: AsyncRead + Unpin,
{
    let mut buffer = [0u8; READ_BUFFER_LEN];

    stream.read_exact(&mut buffer[..V2_MINIMUM_LEN]).await?;
    if buffer[..V2_PREFIX_LEN] != PROTOCOL_SIGNATURE {
        return Err(InvalidProxyProtocolHeader);
    }

    let dynamic_buffer = read_v2_header(&mut stream, &mut buffer).await?;
    // Choose which buffer to parse
    let buffer = dynamic_buffer.as_deref().unwrap_or(&buffer[..]);
    let header = HeaderResult::parse(buffer);

    let connection_info = match header {
        HeaderResult::V1(_) => {
            return Err(Error::Unsupported("Proxy Protocol v1 is not supported"));
        }
        HeaderResult::V2(Ok(header)) => match header.addresses {
            v2::Addresses::IPv4(ip) => Some(ConnectionInfo {
                source_ip: ip.source_address.into(),
                destination_ip: ip.destination_address.into(),
                source_port: ip.source_port,
                destination_port: ip.destination_port,
            }),
            v2::Addresses::IPv6(ip) => Some(ConnectionInfo {
                source_ip: ip.source_address.into(),
                destination_ip: ip.destination_address.into(),
                source_port: ip.source_port,
                destination_port: ip.destination_port,
            }),
            v2::Addresses::Unix(_) => {
                return Err(Error::Unsupported("Unix address is not supported"));
            }
            v2::Addresses::Unspecified => None,
        },
        HeaderResult::V2(Err(err)) => return Err(Error::Parse(err)),
    };

    Ok((stream, connection_info))
}

async fn read_v2_header<I>(
    mut stream: I,
    buffer: &mut [u8; READ_BUFFER_LEN],
) -> Result<Option<Vec<u8>>, std::io::Error>
where
    I: AsyncRead + Unpin,
{
    let length =
        u16::from_be_bytes([buffer[V2_LENGTH_INDEX], buffer[V2_LENGTH_INDEX + 1]]) as usize;
    let full_length = V2_MINIMUM_LEN + length;

    // Switch to dynamic buffer if header is too long; v2 has no maximum length
    if full_length > READ_BUFFER_LEN {
        let mut dynamic_buffer = Vec::with_capacity(full_length);
        dynamic_buffer.extend_from_slice(&buffer[..V2_MINIMUM_LEN]);

        // Read the remaining header length
        stream
            .read_exact(&mut dynamic_buffer[V2_MINIMUM_LEN..full_length])
            .await?;

        Ok(Some(dynamic_buffer))
    } else {
        // Read the remaining header length
        stream
            .read_exact(&mut buffer[V2_MINIMUM_LEN..full_length])
            .await?;

        Ok(None)
    }
}
