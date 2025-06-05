use std::{fs::File, io, sync::Arc};
use std::time::Duration;
use thiserror::Error;
use tokio::{io::AsyncWriteExt, net::TcpListener, select, sync::mpsc::Sender};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer},
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    models::{NewMessage, SmtpCredentialRepository},
    smtp::{
        SmtpConfig,
        connection::{self, ConnectionError},
    },
};

#[derive(Debug, Error)]
pub enum SmtpServerError {
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

pub struct SmtpServer {
    user_repository: SmtpCredentialRepository,
    queue: Sender<NewMessage>,
    shutdown: CancellationToken,
    config: Arc<SmtpConfig>,
}

impl SmtpServer {
    pub fn new(
        config: Arc<SmtpConfig>,
        user_repository: SmtpCredentialRepository,
        queue: Sender<NewMessage>,
        shutdown: CancellationToken,
    ) -> SmtpServer {
        SmtpServer {
            config,
            user_repository,
            queue,
            shutdown,
        }
    }

    async fn load_tls_config(
        &self,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), SmtpServerError> {
        let mut cert_reader = io::BufReader::new(
            File::open(&self.config.cert_file).map_err(SmtpServerError::Certificate)?,
        );
        let mut key_reader = io::BufReader::new(
            File::open(&self.config.key_file).map_err(SmtpServerError::PrivateKey)?,
        );

        let certs = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, io::Error>>()
            .map_err(SmtpServerError::Certificate)?;
        let key = rustls_pemfile::private_key(&mut key_reader)
            .map_err(SmtpServerError::PrivateKey)?
            .ok_or(SmtpServerError::PrivateKeyNotFound)?;

        Ok((certs, key))
    }
    
    async fn build_tls_acceptor(&self) -> Result<TlsAcceptor, SmtpServerError> {
        let (certs, key) = self.load_tls_config().await?;

        let config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(SmtpServerError::Tls)?;

        Ok(TlsAcceptor::from(Arc::new(config)))

    }

    pub async fn serve(self) -> Result<(), SmtpServerError> {
        let listener = TcpListener::bind(&self.config.listen_addr)
            .await
            .map_err(SmtpServerError::Listen)?;
        
        let mut acceptor = self.build_tls_acceptor().await?;
        
        info!("smtp server on {}", self.config.listen_addr);
        loop {
            select! {
                _ = self.shutdown.cancelled() => {
                    info!("shutting down smtp server");

                    return Ok(());
                }
                _ = tokio::time::sleep(Duration::from_secs(100)) => {
                    info!("Reloading TLS config");
                    acceptor = self.build_tls_acceptor().await?;
                }
                result = listener.accept() => match result {
                    Ok((stream, peer_addr)) => {
                        info!("connection from {}", peer_addr);
                        let acceptor = acceptor.clone();
                        let user_repository = self.user_repository.clone();
                        let queue = self.queue.clone();
                        let server_name = self.config.server_name.clone();

                        let task = async move || {
                            let mut tls_stream = acceptor
                                .accept(stream)
                                .await
                                .map_err(ConnectionError::Accept)?;

                            connection::handle(
                                &mut tls_stream,
                                server_name.as_str(),
                                peer_addr,
                                queue,
                                user_repository,
                            )
                            .await?;

                            tls_stream.shutdown().await.map_err(ConnectionError::Write)
                        };

                        tokio::spawn(async {
                            if let Err(err) = task().await {
                                info!("{err}");
                            }
                        });
                    }
                    Err(err) => {
                        error!("failed to accept connection: {}", err);
                    }
                },
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
