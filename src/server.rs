use std::{fs::File, io, net::SocketAddr, path::PathBuf, sync::Arc};
use thiserror::Error;
use tokio::{net::TcpListener, select, sync::mpsc::Sender};
use tokio_rustls::{
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer},
    },
    TlsAcceptor,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{connection::Connection, message::Message, users::UserRepository};

#[derive(Debug, Error)]
pub(crate) enum SmtpServerError {
    #[error("failed to load private key: {0}")]
    PrivateKey(io::Error),
    #[error("no private key found in the key file")]
    PrivateKeyNotFound,
    #[error("failed to load certificate: {0}")]
    Certificate(io::Error),
    #[error("failed to listen on address: {0}")]
    Listen(io::Error),
    #[error("failed to configure TLS: {0}")]
    Tls(rustls::Error),
}

pub(crate) struct SmtServer {
    address: SocketAddr,
    user_repository: UserRepository,
    queue: Sender<Message>,
    shutdown: CancellationToken,
    cert: PathBuf,
    key: PathBuf,
}

impl SmtServer {
    pub fn new(
        address: SocketAddr,
        cert: PathBuf,
        key: PathBuf,
        user_repository: UserRepository,
        queue: Sender<Message>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            address,
            user_repository,
            queue,
            shutdown,
            cert,
            key,
        }
    }

    async fn load_tls_config(
        &self,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), SmtpServerError> {
        let mut cert_reader =
            io::BufReader::new(File::open(&self.cert).map_err(SmtpServerError::Certificate)?);
        let mut key_reader =
            io::BufReader::new(File::open(&self.key).map_err(SmtpServerError::PrivateKey)?);

        let certs = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, io::Error>>()
            .map_err(SmtpServerError::Certificate)?;
        let key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(SmtpServerError::PrivateKey)?
            .ok_or(SmtpServerError::PrivateKeyNotFound)?;

        Ok((certs, key))
    }

    pub async fn serve(self) -> Result<(), SmtpServerError> {
        let (certs, key) = self.load_tls_config().await?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(SmtpServerError::Tls)?;

        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind(&self.address)
            .await
            .map_err(SmtpServerError::Listen)?;

        info!("smtp server on {}:{}", self.address, self.address.port());

        loop {
            select! {
                _ = self.shutdown.cancelled() => {
                    info!("shutting down smtp server");

                    return Ok(());
                }
                result = listener.accept() => {
                    match result {
                        Ok((stream, peer_addr)) => {
                            info!("accepted connection from {}", peer_addr);
                            tokio::spawn(Connection::new(acceptor.clone(), stream, peer_addr).handle(self.queue.clone(), self.user_repository.clone()));
                            info!("connection handled");
                        }
                        Err(err) => {
                            error!("failed to accept connection: {}", err);
                        }
                    }
                }
            }
        }
    }

    pub fn spawn(self) {
        tokio::spawn(async {
            if let Err(e) = self.serve().await {
                error!("smtp server error: {:?}", e);
            }
        });
    }
}
