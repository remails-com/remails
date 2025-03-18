mod connection;
pub mod server;
mod session;

#[cfg(test)]
mod test {
    use crate::{
        models::{Message, SmtpCredential, SmtpCredentialRepository},
        smtp::server::SmtpServer,
        test::random_port,
    };
    use mail_send::{SmtpClientBuilder, mail_builder::MessageBuilder};
    use sqlx::PgPool;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use tokio::{sync::mpsc, task::JoinHandle};
    use tokio_rustls::rustls::crypto;
    use tokio_util::sync::CancellationToken;
    use tracing_test::traced_test;

    async fn setup_server(
        pool: PgPool,
    ) -> (
        CancellationToken,
        JoinHandle<()>,
        mpsc::Receiver<Message>,
        u16,
    ) {
        let smtp_port = random_port();
        let user_repository = SmtpCredentialRepository::new(pool);

        let credential = SmtpCredential::new(
            "john".into(),
            "p4ssw0rd".into(),
            "test-org-1.com".to_string(),
        );
        user_repository.create(&credential).await.unwrap();

        let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), smtp_port);
        let shutdown = CancellationToken::new();
        let (queue_sender, receiver) = mpsc::channel::<Message>(100);
        let server = SmtpServer::new(
            socket,
            "cert.pem".into(),
            "key.pem".into(),
            user_repository,
            queue_sender,
            shutdown.clone(),
        );

        let server_handle = tokio::spawn(async move {
            server.serve().await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        (shutdown, server_handle, receiver, smtp_port)
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "domains")))]
    #[traced_test]
    async fn test_smtp(pool: PgPool) {
        if crypto::CryptoProvider::get_default().is_none() {
            crypto::aws_lc_rs::default_provider()
                .install_default()
                .expect("Failed to install crypto provider")
        }

        let (shutdown, server_handle, mut receiver, port) = setup_server(pool).await;

        let message = MessageBuilder::new()
            .from(("John Doe", "john@example.com"))
            .to(vec![
                ("Jane Doe", "jane@example.com"),
                ("James Smith", "james@test.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!");

        SmtpClientBuilder::new("localhost", port)
            .implicit_tls(true)
            .allow_invalid_certs()
            .credentials(("john", "p4ssw0rd"))
            .connect()
            .await
            .unwrap()
            .send(message)
            .await
            .unwrap();

        shutdown.cancel();
        server_handle.await.unwrap();

        let received_message = receiver.recv().await.unwrap();
        assert_eq!(received_message.from_email, "john@example.com");
    }

    #[sqlx::test(fixtures(path = "../fixtures", scripts("organizations", "domains")))]
    #[traced_test]
    async fn test_smtp_wrong_credentials(pool: PgPool) {
        let (shutdown, server_handle, _, port) = setup_server(pool).await;

        let result = SmtpClientBuilder::new("localhost", port)
            .implicit_tls(true)
            .allow_invalid_certs()
            .credentials(("john", "wrong"))
            .connect()
            .await;

        assert!(result.is_err());

        shutdown.cancel();
        server_handle.await.unwrap();
    }
}
