use crate::{
    handler::HandlerConfig,
    models::{Message, SmtpCredential, SmtpCredentialResponse},
    run_api_server, run_mta,
    smtp::SmtpConfig,
};
use http::{HeaderMap, header, header::CONTENT_TYPE};
use mail_send::{SmtpClientBuilder, mail_builder::MessageBuilder};
use mailcrab::TestMailServerHandle;
use rand::Rng;
use serde_json::json;
use serial_test::serial;
use sqlx::PgPool;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::select;
use tracing_test::traced_test;

pub fn random_port() -> u16 {
    let mut rng = rand::rng();

    rng.random_range(10_000..30_000)
}

#[sqlx::test(fixtures("organizations", "domains", "api_users"))]
#[traced_test]
#[serial]
async fn integration_test(pool: PgPool) {
    let client = reqwest::ClientBuilder::new()
        .default_headers(HeaderMap::from_iter([(
            CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        )]))
        .cookie_store(true)
        .build()
        .unwrap();

    let smtp_port = random_port();
    let mailcrab_random_port = random_port();
    let http_port = random_port();

    let smtp_socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), smtp_port);
    let http_socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), http_port);

    let TestMailServerHandle {
        token,
        rx: mut mailcrab_rx,
    } = mailcrab::development_mail_server(Ipv4Addr::new(127, 0, 0, 1), mailcrab_random_port).await;

    let smtp_config = SmtpConfig {
        listen_addr: smtp_socket.into(),
        server_name: "localhost".to_string(),
        cert_file: "cert.pem".into(),
        key_file: "key.pem".into(),
    };

    let handler_config = HandlerConfig {
        domain: "test".to_string(),
        resolver: crate::handler::mock::Resolver("localhost", mailcrab_random_port),
    };

    run_mta(pool.clone(), smtp_config, handler_config, token.clone()).await;
    run_api_server(pool, http_socket, token.clone(), false).await;

    let _drop_guard = token.drop_guard();

    let john_pwd = client
        .post(format!(
            "http://localhost:{}/api/smtp_credentials",
            http_port
        ))
        .header("X-Test-Login", "admin")
        .json(&json!({
            "username": "john",
            "domain_id": "ed28baa5-57f7-413f-8c77-7797ba6a8780"
        }))
        .send()
        .await
        .unwrap()
        .json::<SmtpCredentialResponse>()
        .await
        .unwrap()
        .cleartext_password();

    let eddy_pwd = client
        .post(format!(
            "http://localhost:{}/api/smtp_credentials",
            http_port
        ))
        .header("X-Test-Login", "admin")
        .json(&json!({
            "username": "eddy",
            "domain_id": "6a45a141-6628-4c0f-823b-3cf3eb64f0c7"
        }))
        .send()
        .await
        .unwrap()
        .json::<SmtpCredentialResponse>()
        .await
        .unwrap()
        .cleartext_password();

    let credentials: Vec<SmtpCredential> = client
        .get(format!(
            "http://localhost:{}/api/smtp_credentials",
            http_port
        ))
        .header("X-Test-Login", "admin")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(credentials.len(), 2);

    let mut john_smtp_client = SmtpClientBuilder::new("localhost", smtp_port)
        .implicit_tls(true)
        .allow_invalid_certs()
        .credentials(("john", john_pwd.as_str()))
        .connect()
        .await
        .unwrap();

    for i in 1..=10 {
        let message = MessageBuilder::new()
            .from(("John", "john@example.com"))
            .to(vec![("Eddy", "eddy@example.com")])
            .subject("TPS reports")
            .text_body(format!(
                "Have you finished the TPS reports yet? This is the {i}th reminder!!!"
            ));
        john_smtp_client.send(message).await.unwrap();

        select! {
            Ok(recv) = mailcrab_rx.recv() => {
                assert_eq!(recv.envelope_from.as_str(), "john@example.com");
                assert_eq!(recv.envelope_recipients.len(), 1);
                assert_eq!(recv.envelope_recipients[0].as_str(), "eddy@example.com");
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => panic!("timed out receiving email"),
        }
    }

    let message = MessageBuilder::new()
        .from(("Eddy", "eddy@example.com"))
        .to(vec![
            ("John", "john@example.com"),
        ])
        .subject("Re: TPS reports")
        .text_body("Ah! Yeah. It's just we're putting new coversheets on all the TPS reports before they go out now.
        So if you could go ahead and try to remember to do that from now on, that'd be great. All right!");

    SmtpClientBuilder::new("localhost", smtp_port)
        .implicit_tls(true)
        .allow_invalid_certs()
        .credentials(("eddy", eddy_pwd.as_str()))
        .connect()
        .await
        .unwrap()
        .send(message)
        .await
        .unwrap();

    select! {
        Ok(recv) = mailcrab_rx.recv() => {
            assert_eq!(recv.envelope_from.as_str(), "eddy@example.com");
            assert_eq!(recv.envelope_recipients.len(), 1);
            assert_eq!(recv.envelope_recipients[0].as_str(), "john@example.com");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => panic!("timed out receiving email"),
    }

    let messages: Vec<Message> = client
        .get(format!("http://localhost:{}/api/messages", http_port))
        .header("X-Test-Login", "44729d9f-a7dc-4226-b412-36a7537f5176")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 10);

    let messages: Vec<Message> = client
        .get(format!("http://localhost:{}/api/messages", http_port))
        // Non-existent organization
        .header("X-Test-Login", "ab5647ee-ea7c-40f8-ad70-bdcbff7fa4cd")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 0);

    let messages: Vec<Message> = client
        .get(format!("http://localhost:{}/api/messages", http_port))
        .header("X-Test-Login", "admin")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 11);
}
