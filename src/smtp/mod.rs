use crate::{Environment, handler::RetryConfig};
use std::{env, path::PathBuf};

mod connection;
mod proxy_protocol;
pub mod server;
mod session;

#[derive(Clone)]
pub struct SmtpConfig {
    pub listen_addr: core::net::SocketAddr,
    pub server_name: String,
    pub cert_file: PathBuf,
    pub key_file: PathBuf,
    pub environment: Environment,
    pub retry: RetryConfig,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        let listen_addr = env::var("SMTP_LISTEN_ADDR")
            .expect("Missing SMTP_LISTEN_ADDR environment variable")
            .parse()
            .expect("Invalid SMTP_LISTEN_ADDR");
        let server_name =
            env::var("SMTP_SERVER_NAME").expect("Missing SMTP_SERVER_NAME environment variable");
        let cert_file = env::var("SMTP_CERT_FILE")
            .expect("Missing SMTP_CERT_FILE environment variable")
            .parse()
            .expect("Invalid SMTP_CERT_FILE path");
        let key_file = env::var("SMTP_KEY_FILE")
            .expect("Missing SMTP_KEY_FILE environment variable")
            .parse()
            .expect("Invalid SMTP_KEY_FILE path");

        Self {
            listen_addr,
            server_name,
            cert_file,
            key_file,
            environment: Environment::from_env(),
            retry: Default::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        bus::client::BusClient,
        models::{
            Label, MessageRepository, MessageStatus, SmtpCredentialRepository,
            SmtpCredentialRequest,
        },
        smtp::{SmtpConfig, server::SmtpServer},
        test::{TestProjects, random_port},
    };
    use mail_builder::headers::text::Text;
    use mail_parser::MessageParser;
    use mail_send::{SmtpClientBuilder, mail_builder::MessageBuilder};
    use sqlx::PgPool;
    use std::{
        net::{Ipv4Addr, SocketAddrV4},
        sync::Arc,
    };
    use tokio::task::JoinHandle;
    use tokio_util::sync::CancellationToken;

    async fn setup_server(
        pool: PgPool,
    ) -> (CancellationToken, JoinHandle<()>, u16, String, String) {
        let smtp_port = random_port();

        let (org_id, project_id) = TestProjects::Org1Project1.get_ids();

        let credential_request = SmtpCredentialRequest {
            username: "john".to_string(),
            description: "Test SMTP credential description".to_string(),
        };

        let credential_repo = SmtpCredentialRepository::new(pool.clone());
        let credential = credential_repo
            .generate(org_id, project_id, &credential_request)
            .await
            .unwrap();

        let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), smtp_port);
        let config = Arc::new(SmtpConfig {
            listen_addr: socket.into(),
            server_name: "localhost".to_string(),
            cert_file: "dev-secrets/cert.pem".into(),
            key_file: "dev-secrets/key.pem".into(),
            ..Default::default()
        });
        let shutdown = CancellationToken::new();
        let bus_client = BusClient::new_from_env_var().unwrap();
        let server = SmtpServer::new(pool, config, bus_client, shutdown.clone());

        let server_handle = tokio::spawn(async move {
            server.serve().await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        (
            shutdown,
            server_handle,
            smtp_port,
            credential.username(),
            credential.cleartext_password(),
        )
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "k8s_nodes"
        )
    ))]
    async fn test_smtp(pool: PgPool) {
        let (shutdown, server_handle, port, username, pwd) = setup_server(pool.clone()).await;

        let message = MessageBuilder::new()
            .from(("John Doe", "john@test-org-1-project-1.com"))
            .to(vec![
                ("Jane Doe", "jane@test-org-1-project-1.com"),
                ("James Smith", "james@test.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!")
            .message_id("83f667c7-1da8-4062-a936-fbfab899365b@my-custom-id")
            .header("X-remails-LabeL", Text::new("my label"));

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

        // message should now be received and stored in the database
        let org_id = TestProjects::Org1Project1.org_id();
        let messages = MessageRepository::new(pool);
        let received_messages = messages
            .list_message_metadata(org_id, Default::default())
            .await
            .unwrap();
        assert_eq!(received_messages.len(), 1);
        assert_eq!(received_messages[0].status, MessageStatus::Processing);
        assert_eq!(
            received_messages[0].from_email,
            "john@test-org-1-project-1.com".parse().unwrap()
        );
        assert_eq!(
            received_messages[0].message_id_header,
            "83f667c7-1da8-4062-a936-fbfab899365b@my-custom-id".to_owned(),
        );
        assert_eq!(received_messages[0].label, Some(Label::new("my-label")));
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "k8s_nodes"
        )
    ))]
    async fn test_missing_headers_get_added(pool: PgPool) {
        let (shutdown, server_handle, port, username, pwd) = setup_server(pool.clone()).await;

        // message without Message-ID or Date
        let message = "From: \"John Doe\" <john@test-org-1-project-1.com>\r\n\
            To: \"Jane Doe\" <jane@test-org-1-project-1.com>\r\n\
            Subject: Hi!\r\n\
            X-remails-LabeL: my label \r\n\
            MIME-Version: 1.0\r\n\
            Content-Type: text/plain; charset=\"utf-8\"\r\n\
            Content-Transfer-Encoding: 7bit\r\n\
            \r\n\
            Hello world!";

        let message = MessageParser::default().parse(message).unwrap();

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

        // message should now be received and stored in the database
        let org_id = TestProjects::Org1Project1.org_id();
        let messages = MessageRepository::new(pool);
        let received_messages = messages
            .list_message_metadata(org_id, Default::default())
            .await
            .unwrap();
        assert_eq!(received_messages.len(), 1);
        assert_eq!(received_messages[0].status, MessageStatus::Processing);
        assert_eq!(
            received_messages[0].from_email,
            "john@test-org-1-project-1.com".parse().unwrap()
        );
        assert_eq!(
            received_messages[0].message_id_header,
            // automatically generated Message ID header
            format!(
                "REMAILS-{}@test-org-1-project-1.com",
                received_messages[0].id
            ),
        );
        assert_eq!(received_messages[0].label, Some(Label::new("my-label")));

        // raw data should have the missing headers added
        let message = messages
            .find_by_id(org_id, received_messages[0].id)
            .await
            .unwrap();
        assert!(message.truncated_raw_data.contains("Date: "));
        assert!(message.truncated_raw_data.contains("Message-ID: "));
    }

    #[sqlx::test(fixtures(
        path = "../fixtures",
        scripts("organizations", "projects", "org_domains", "proj_domains")
    ))]
    async fn test_smtp_wrong_credentials(pool: PgPool) {
        let (shutdown, server_handle, port, username, _) = setup_server(pool).await;

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
