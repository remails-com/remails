use crate::{
    handler::{HandlerConfig, dns::DnsResolver},
    models::{ApiMessageMetadata, SmtpCredential, SmtpCredentialResponse},
    run_api_server, run_mta,
    smtp::SmtpConfig,
};
use http::{HeaderMap, StatusCode, header, header::CONTENT_TYPE};
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

#[sqlx::test(fixtures("organizations", "api_users", "projects", "domains", "streams"))]
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
        allow_plain: true,
        domain: "test".to_string(),
        resolver: DnsResolver::mock("localhost", mailcrab_random_port),
        retry_delay: chrono::Duration::minutes(5),
        max_retries: 1,
    };

    run_mta(pool.clone(), smtp_config, handler_config, token.clone()).await;
    run_api_server(pool, http_socket, token.clone(), false).await;

    let _drop_guard = token.drop_guard();

    let org_id = "44729d9f-a7dc-4226-b412-36a7537f5176";
    let project_id = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462";
    let stream_id = "85785f4c-9167-4393-bbf2-3c3e21067e4a";

    let john_cred = client
        .post(format!(
            "http://localhost:{}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials",
            http_port
        ))
        .header("X-Test-Login", org_id)
        .json(&json!({
            "username": "john",
            "description": "John test credential"
        }))
        .send()
        .await
        .unwrap()
        .json::<SmtpCredentialResponse>()
        .await
        .unwrap();

    let org_id = "5d55aec5-136a-407c-952f-5348d4398204";
    let project_id = "70ded685-8633-46ef-9062-d9fbad24ae95";
    let stream_id = "6af665cd-698e-47ca-9d6b-966f8e8fa07f";

    let eddy_cred = client
        .post(format!(
            "http://localhost:{}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials",
            http_port
        ))
        .header("X-Test-Login", org_id)
        .json(&json!({
            "username": "eddy",
            "description": "Eddy test credential"
        }))
        .send()
        .await
        .unwrap()
        .json::<SmtpCredentialResponse>()
        .await
        .unwrap();

    let credentials: Vec<SmtpCredential> = client
        .get(format!(
            "http://localhost:{}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials",
            http_port
        ))
        .header("X-Test-Login", org_id)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(credentials.len(), 1);

    let mut john_smtp_client = SmtpClientBuilder::new("localhost", smtp_port)
        .implicit_tls(true)
        .allow_invalid_certs()
        .credentials((
            john_cred.username().as_str(),
            john_cred.cleartext_password().as_str(),
        ))
        .connect()
        .await
        .unwrap();

    for i in 1..=10 {
        let message = MessageBuilder::new()
            .from(("John", "john@test-org-1-project-1.com"))
            .to(vec![("Eddy", "eddy@test-org-2-project-1.com")])
            .subject("TPS reports")
            .text_body(format!(
                "Have you finished the TPS reports yet? This is the {i}th reminder!!!"
            ));
        john_smtp_client.send(message).await.unwrap();

        select! {
            Ok(recv) = mailcrab_rx.recv() => {
                assert_eq!(recv.envelope_from.as_str(), "john@test-org-1-project-1.com");
                assert_eq!(recv.envelope_recipients.len(), 1);
                assert_eq!(recv.envelope_recipients[0].as_str(), "eddy@test-org-2-project-1.com");
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => panic!("timed out receiving email"),
        }
    }

    let message = MessageBuilder::new()
        .from(("Eddy", "eddy@test-org-2-project-1.com"))
        .to(vec![
            ("John", "john@test-org-1-project-1.com"),
        ])
        .subject("Re: TPS reports")
        .text_body("Ah! Yeah. It's just we're putting new coversheets on all the TPS reports before they go out now.
        So if you could go ahead and try to remember to do that from now on, that'd be great. All right!");

    SmtpClientBuilder::new("localhost", smtp_port)
        .implicit_tls(true)
        .allow_invalid_certs()
        .credentials((
            eddy_cred.username().as_str(),
            eddy_cred.cleartext_password().as_str(),
        ))
        .connect()
        .await
        .unwrap()
        .send(message)
        .await
        .unwrap();

    select! {
        Ok(recv) = mailcrab_rx.recv() => {
            assert_eq!(recv.envelope_from.as_str(), "eddy@test-org-2-project-1.com");
            assert_eq!(recv.envelope_recipients.len(), 1);
            assert_eq!(recv.envelope_recipients[0].as_str(), "john@test-org-1-project-1.com");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => panic!("timed out receiving email"),
    }

    let org_id = "44729d9f-a7dc-4226-b412-36a7537f5176";
    let project_id = "3ba14adf-4de1-4fb6-8c20-50cc2ded5462";
    let stream_id = "85785f4c-9167-4393-bbf2-3c3e21067e4a";
    let messages: Vec<ApiMessageMetadata> = client
        .get(format!("http://localhost:{http_port}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/messages"))
        .header("X-Test-Login", "44729d9f-a7dc-4226-b412-36a7537f5176")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 10);

    let status = client
        .get(format!("http://localhost:{http_port}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/messages"))
        // Non-existent organization
        .header("X-Test-Login", "ab5647ee-ea7c-40f8-ad70-bdcbff7fa4cd")
        .send()
        .await
        .unwrap()
        .status();

    assert_eq!(status, StatusCode::FORBIDDEN);
}
