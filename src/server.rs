use anyhow::Context;
use std::{fs::File, io, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{net::TcpListener, select, sync::mpsc::Sender};
use tokio_rustls::{
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer},
    },
    TlsAcceptor,
};
use tokio_util::sync::CancellationToken;

use crate::{connection::Connection, message::Message, users::UserRepository};

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
    ) -> anyhow::Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        let mut cert_reader = io::BufReader::new(File::open(&self.cert)?);
        let mut key_reader = io::BufReader::new(File::open(&self.key)?);

        let certs =
            rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, io::Error>>()?;
        let key = rustls_pemfile::private_key(&mut key_reader)?.context("No key found")?;

        Ok((certs, key))
    }

    pub async fn serve(&self) -> anyhow::Result<()> {
        let (certs, key) = self.load_tls_config().await?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind(&self.address).await?;

        tracing::info!("Listening on {}:{}", self.address, self.address.port());

        loop {
            select! {
                _ = self.shutdown.cancelled() => {
                    tracing::info!("Shutting down server");

                    return Ok(());
                }
                result = listener.accept() => {
                    match result {
                        Ok((stream, peer_addr)) => {
                            tracing::info!("Accepted connection from {}", peer_addr);

                            tokio::spawn(Connection::new(acceptor.clone(), stream, peer_addr).handle(self.queue.clone(), self.user_repository.clone()));

                            tracing::info!("Connection handled");
                        }
                        Err(err) => {
                            tracing::error!("Failed to accept connection: {}", err);
                        }
                    }
                }
            }
        }
    }
}
