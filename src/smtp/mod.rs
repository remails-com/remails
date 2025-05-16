use std::path::PathBuf;

mod connection;
pub mod server;
mod session;

pub struct SmtpConfig {
    pub listen_addr: core::net::SocketAddr,
    pub server_name: String,
    pub cert_file: PathBuf,
    pub key_file: PathBuf,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        let listen_addr = std::env::var("SMTP_LISTEN_ADDR")
            .expect("Missing SMTP_LISTEN_ADDR environment variable")
            .parse()
            .expect("Invalid SMTP_LISTEN_ADDR");
        let server_name = std::env::var("SMTP_SERVER_NAME")
            .expect("Missing SMTP_SERVER_NAME environment variable");
        let cert_file = std::env::var("SMTP_CERT_FILE")
            .expect("Missing SMTP_CERT_FILE environment variable")
            .parse()
            .expect("Invalid SMTP_CERT_FILE path");
        let key_file = std::env::var("SMTP_KEY_FILE")
            .expect("Missing SMTP_KEY_FILE environment variable")
            .parse()
            .expect("Invalid SMTP_KEY_FILE path");

        Self {
            listen_addr,
            server_name,
            cert_file,
            key_file,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        models::{NewMessage, SmtpCredentialRepository, SmtpCredentialRequest},
        smtp::{SmtpConfig, server::SmtpServer},
        test::random_port,
    };
    use mail_send::{SmtpClientBuilder, mail_builder::MessageBuilder};
    use sqlx::PgPool;
    use std::{
        net::{Ipv4Addr, SocketAddrV4},
        sync::Arc,
    };
    use tokio::{sync::mpsc, task::JoinHandle};
    use tokio_rustls::rustls::crypto;
    use tokio_util::sync::CancellationToken;
    use tracing_test::traced_test;

    async fn setup_server(
        pool: PgPool,
    ) -> (
        CancellationToken,
        JoinHandle<()>,
        mpsc::Receiver<NewMessage>,
        u16,
        String,
        String,
    ) {
        let smtp_port = random_port();
        let user_repository = SmtpCredentialRepository::new(pool.clone());

        let org_id = "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap();
        let project_id = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap();
        let stream_id = "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap();

        let credential_request = SmtpCredentialRequest {
            username: "john".to_string(),
            description: "Test SMTP credential description".to_string(),
        };

        let credential_repo = SmtpCredentialRepository::new(pool.clone());
        let credential = credential_repo
            .generate(org_id, project_id, stream_id, &credential_request)
            .await
            .unwrap();

        let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), smtp_port);
        let config = Arc::new(SmtpConfig {
            listen_addr: socket.into(),
            server_name: "localhost".to_string(),
            cert_file: "cert.pem".into(),
            key_file: "key.pem".into(),
        });
        let shutdown = CancellationToken::new();
        let (queue_sender, receiver) = mpsc::channel::<NewMessage>(100);
        let server = SmtpServer::new(config, user_repository, queue_sender, shutdown.clone());

        let server_handle = tokio::spawn(async move {
            server.serve().await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        (
            shutdown,
            server_handle,
            receiver,
            smtp_port,
            credential.username(),
            credential.cleartext_password(),
        )
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "domains", "streams")
    ))]
    #[traced_test]
    async fn test_smtp(pool: PgPool) {
        if crypto::CryptoProvider::get_default().is_none() {
            crypto::aws_lc_rs::default_provider()
                .install_default()
                .expect("Failed to install crypto provider")
        }

        let (shutdown, server_handle, mut receiver, port, username, pwd) = setup_server(pool).await;

        let message = MessageBuilder::new()
            .from(("John Doe", "john@test-org-1-project-1.com"))
            .to(vec![
                ("Jane Doe", "jane@test-org-1-project-1.com"),
                ("James Smith", "james@test.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!");

        SmtpClientBuilder::new("localhost", port)
            .implicit_tls(true)
            .allow_invalid_certs()
            .credentials((username.as_str(), pwd.as_str()))
            .connect()
            .await
            .unwrap()
            .send(message)
            .await
            .unwrap();

        shutdown.cancel();
        server_handle.await.unwrap();

        let received_message = receiver.recv().await.unwrap();
        assert_eq!(
            received_message.from_email,
            "john@test-org-1-project-1.com".parse().unwrap()
        );
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "domains", "streams")
    ))]
    #[traced_test]
    async fn test_smtp_wrong_credentials(pool: PgPool) {
        let (shutdown, server_handle, _, port, username, _) = setup_server(pool).await;

        let result = SmtpClientBuilder::new("localhost", port)
            .implicit_tls(true)
            .allow_invalid_certs()
            .credentials((username.as_str(), "wrong"))
            .connect()
            .await;

        assert!(result.is_err());

        shutdown.cancel();
        server_handle.await.unwrap();
    }
}
