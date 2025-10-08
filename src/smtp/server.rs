use crate::{
    Environment,
    messaging::BusClient,
    models::{MessageRepository, SmtpCredentialRepository},
    smtp::{
        SmtpConfig,
        connection::{self, ConnectionError},
        proxy_protocol::{self, Error, handle_proxy_protocol},
    },
};
use rand::random_range;
use sqlx::PgPool;
use std::{fs::File, io, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{io::AsyncWriteExt, net::TcpListener, select, sync::RwLock};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        self,
        pki_types::{CertificateDer, PrivateKeyDer},
    },
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace};

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
    #[error("{0}")]
    ProxyProtocol(#[from] proxy_protocol::Error),
}

pub struct SmtpServer {
    user_repository: SmtpCredentialRepository,
    message_repository: MessageRepository,
    bus_client: BusClient,
    shutdown: CancellationToken,
    config: Arc<SmtpConfig>,
}

impl SmtpServer {
    pub fn new(pool: PgPool, config: Arc<SmtpConfig>, shutdown: CancellationToken) -> SmtpServer {
        SmtpServer {
            user_repository: SmtpCredentialRepository::new(pool.clone()),
            message_repository: MessageRepository::new(pool),
            bus_client: BusClient::new_from_env_var().unwrap(),
            shutdown,
            config,
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
        let environment = self.config.environment;
        let listener = TcpListener::bind(&self.config.listen_addr)
            .await
            .map_err(SmtpServerError::Listen)?;

        let acceptor = Arc::new(RwLock::new(self.build_tls_acceptor().await?));

        info!("smtp server on {}", self.config.listen_addr);

        let certificate_reload_interval =
            Duration::from_secs(60 * 60 * 23 + random_range(0..(60 * 60)));
        debug!(
            "Automatically reloading the SMTP certificate every {:?}",
            certificate_reload_interval
        );

        let server_name = self.config.server_name.clone();
        let bus_client = self.bus_client.clone();
        let user_repository = self.user_repository.clone();
        let message_repository = self.message_repository.clone();
        let max_automatic_retries = self.config.retry.max_automatic_retries;
        let shutdown = self.shutdown.clone();

        let acceptor_clone = acceptor.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(certificate_reload_interval);
            loop {
                interval.tick().await;
                let mut a = acceptor_clone.write().await;
                info!("Reloading the SMTP TLS certificate");
                *a = self.build_tls_acceptor().await.unwrap();
            }
        });

        loop {
            select! {
                _ = shutdown.cancelled() => {
                    info!("shutting down smtp server");

                    return Ok(());
                }
                result = listener.accept() => match result {
                    Ok((mut stream, peer_addr)) => {

                        let mut connection_info = None;
                        if !matches!(environment, Environment::Development) {
                            (stream, connection_info) = match handle_proxy_protocol(stream).await {
                                Ok((stream, connection_info)) => {(stream, connection_info)}
                                Err(err) => {
                                    if matches!(err, Error::Io(_)) {
                                        trace!("failed to read the proxy protocol: {err}")
                                    } else {
                                        error!("failed to read the proxy protocol: {err}")
                                    }
                                    continue;
                                }
                            }
                        }

                        if let Some(connection_info) = connection_info{
                            info!(
                                source_ip=connection_info.source_ip.to_string(),
                                source_port=connection_info.source_port,
                                destination_ip=connection_info.destination_ip.to_string(),
                                destination_port=connection_info.destination_port,
                                "new TCP connection"
                            )
                        } else {
                            trace!(
                                source_ip=peer_addr.ip().to_string(),
                                source_port=peer_addr.port(),
                                "new TCP connection"
                            );
                        }
                        let acceptor = acceptor.clone();
                        let server_name = server_name.clone();
                        let bus_client = bus_client.clone();
                        let user_repository = user_repository.clone();
                        let message_repository = message_repository.clone();

                        let task = async move || {
                            let mut tls_stream = acceptor.read().await
                                .accept(stream)
                                .await
                                .map_err(ConnectionError::Accept)?;

                            connection::handle(
                                &mut tls_stream,
                                server_name,
                                peer_addr,
                                bus_client,
                                user_repository,
                                message_repository,
                                max_automatic_retries,
                            )
                            .await?;

                            tls_stream.shutdown().await.map_err(ConnectionError::Write)
                        };

                        tokio::spawn(async {
                            if let Err(err) = task().await {
                                let error_string = err.to_string();
                                if let ConnectionError::Accept(e) = err
                                    && (e.kind() == io::ErrorKind::UnexpectedEof || e.kind() == io::ErrorKind::ConnectionReset) {
                                        trace!("failed to handle connection: {error_string}");
                                        return
                                    }
                                error!("failed to handle connection: {error_string}");
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
