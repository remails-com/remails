use crate::{
    Environment,
    bus::{client::BusClient, server::Bus},
    handler::{HandlerConfig, RetryConfig, dns::DnsResolver},
    models::{
        ApiMessageMetadata, OrganizationId, ProjectId, SmtpCredential, SmtpCredentialResponse,
        StreamId,
    },
    run_api_server, run_mta,
    smtp::SmtpConfig,
};
use http::{HeaderMap, StatusCode, header, header::CONTENT_TYPE};
use mail_send::{SmtpClientBuilder, mail_builder::MessageBuilder};
use mailcrab::TestMailServerHandle;
use rand::Rng;
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::{select, task::JoinSet};

pub fn random_port() -> u16 {
    let mut rng = rand::rng();

    rng.random_range(10_000..30_000)
}

/// Streams used for testing, as configured in the fixtures
#[allow(dead_code)]
pub enum TestStreams {
    Org1Project1Stream1,
    Org1Project2Stream1,
    Org1Project2Stream2,
    Org2Project1Stream1,
}

impl TestStreams {
    pub fn stream_id(&self) -> StreamId {
        match self {
            TestStreams::Org1Project1Stream1 => {
                "85785f4c-9167-4393-bbf2-3c3e21067e4a".parse().unwrap()
            }
            TestStreams::Org1Project2Stream1 => {
                "d01de497-b40a-4795-a92e-5a8b83dea565".parse().unwrap()
            }
            TestStreams::Org1Project2Stream2 => {
                "e1bcdb8e-6a01-4f6f-b4fd-2b71f872bb02".parse().unwrap()
            }
            TestStreams::Org2Project1Stream1 => {
                "6af665cd-698e-47ca-9d6b-966f8e8fa07f".parse().unwrap()
            }
        }
    }

    pub fn project_id(&self) -> ProjectId {
        match self {
            TestStreams::Org1Project1Stream1 => {
                "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap()
            }
            TestStreams::Org1Project2Stream1 | TestStreams::Org1Project2Stream2 => {
                "da12d059-d86e-4ac6-803d-d013045f68ff".parse().unwrap()
            }
            TestStreams::Org2Project1Stream1 => {
                "70ded685-8633-46ef-9062-d9fbad24ae95".parse().unwrap()
            }
        }
    }

    pub fn org_id(&self) -> OrganizationId {
        match self {
            TestStreams::Org1Project1Stream1
            | TestStreams::Org1Project2Stream1
            | TestStreams::Org1Project2Stream2 => {
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap()
            }
            TestStreams::Org2Project1Stream1 => {
                "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap()
            }
        }
    }

    pub fn get_ids(&self) -> (OrganizationId, ProjectId, StreamId) {
        (self.org_id(), self.project_id(), self.stream_id())
    }

    pub fn get_stringified_ids(&self) -> (String, String, String) {
        (
            self.org_id().to_string(),
            self.project_id().to_string(),
            self.stream_id().to_string(),
        )
    }
}

async fn setup(
    pool: PgPool,
) -> (
    tokio_util::sync::DropGuard,
    reqwest::Client,
    u16,
    tokio::sync::broadcast::Receiver<mailcrab::MailMessage>,
    u16,
) {
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
        rx: mailcrab_rx,
    } = mailcrab::development_mail_server(Ipv4Addr::new(127, 0, 0, 1), mailcrab_random_port).await;

    let retry_config = RetryConfig {
        delay: chrono::Duration::minutes(5),
        max_automatic_retries: 2,
    };

    let smtp_config = SmtpConfig {
        listen_addr: smtp_socket.into(),
        server_name: "localhost".to_string(),
        cert_file: "cert.pem".into(),
        key_file: "key.pem".into(),
        environment: Default::default(),
        retry: retry_config.clone(),
    };

    let handler_config = HandlerConfig {
        allow_plain: true,
        domain: "test".to_string(),
        resolver: DnsResolver::mock("localhost", mailcrab_random_port),
        environment: Environment::Development,
        retry: retry_config,
    };

    let bus_port = Bus::spawn_random_port().await;
    let bus_client = BusClient::new(bus_port, "localhost".to_owned()).unwrap();
    run_mta(
        pool.clone(),
        smtp_config,
        handler_config,
        bus_client,
        token.clone(),
    )
    .await;
    run_api_server(pool, http_socket, token.clone(), false).await;

    let _drop_guard = token.drop_guard();

    (_drop_guard, client, http_port, mailcrab_rx, smtp_port)
}

#[sqlx::test(fixtures(
    "organizations",
    "api_users",
    "projects",
    "org_domains",
    "proj_domains",
    "streams",
    "k8s_nodes"
))]
async fn integration_test(pool: PgPool) {
    // dotenv().unwrap();
    //
    // tracing_subscriber::registry()
    //     .with(
    //         tracing_subscriber::EnvFilter::try_from_default_env()
    //             .unwrap_or("remails=trace,tower_http=debug,axum=trace".parse().unwrap()),
    //     )
    //     .with(
    //         tracing_subscriber::fmt::layer()
    //             .with_file(true)
    //             .with_line_number(true)
    //             .without_time(),
    //     )
    //     .init();

    let (_drop_guard, client, http_port, mut mailcrab_rx, smtp_port) = setup(pool).await;

    let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_stringified_ids();

    let john_cred = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials"
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

    let (org_id, project_id, stream_id) = TestStreams::Org2Project1Stream1.get_stringified_ids();

    let eddy_cred = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials"
        ))
        .header("X-Test-Login", &org_id)
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
            "http://localhost:{http_port}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials"
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
    }

    for i in 1..=10 {
        select! {
            Ok(recv) = mailcrab_rx.recv() => {
                assert_eq!(recv.envelope_from.as_str(), "john@test-org-1-project-1.com");
                assert_eq!(recv.envelope_recipients.len(), 1);
                assert_eq!(recv.envelope_recipients[0].as_str(), "eddy@test-org-2-project-1.com");
            }
            _ = tokio::time::sleep(Duration::from_secs(1)) => panic!("timed out receiving {i}th email"),
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

    let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_stringified_ids();
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

#[sqlx::test(fixtures(
    "organizations",
    "api_users",
    "projects",
    "org_domains",
    "proj_domains",
    "streams",
    "k8s_nodes"
))]
async fn quotas_count_atomically(pool: PgPool) {
    let pool = PgPoolOptions::new()
        .max_connections(70)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();

    let (_drop_guard, client, http_port, mut mailcrab_rx, smtp_port) = setup(pool).await;

    let (org_id, project_id, stream_id) = TestStreams::Org1Project1Stream1.get_stringified_ids();

    let john_cred = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials"
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

    let mut join_set = JoinSet::new();

    join_set.spawn(async move {
        for i in 1.. {
            select! {
                Ok(recv) = mailcrab_rx.recv() => {
                    assert_eq!(recv.envelope_from.as_str(), "john@test-org-1-project-1.com");
                    assert_eq!(recv.envelope_recipients.len(), 1);
                    assert_eq!(recv.envelope_recipients[0].as_str(), "eddy@test-org-2-project-1.com");
                    if i > 800 {
                        panic!("went over quota")
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    if i < 800 {
                        panic!("timed out receiving {i}th email")
                    }
                    return
                },
            }
        }
    });

    // Spawn 51 tasks to throw 100 messages each to remails
    for i in 0..11 {
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

        join_set.spawn(async move {
            for j in 1..=100 {
                let message = MessageBuilder::new()
                    .from(("John", "john@test-org-1-project-1.com"))
                    .to(vec![("Eddy", "eddy@test-org-2-project-1.com")])
                    .subject("TPS reports")
                    .text_body(format!(
                        "Have you finished the TPS reports yet? This is the {}th reminder!!!",
                        i * 50 + j
                    ));
                john_smtp_client.send(message).await.unwrap();
            }
            john_smtp_client.quit().await.unwrap();
        });
    }

    join_set.join_all().await;
}

#[sqlx::test(fixtures(
    "organizations",
    "api_users",
    "projects",
    "org_domains",
    "proj_domains",
    "streams",
    "k8s_nodes"
))]
async fn rate_limit_count_atomically(pool: PgPool) {
    let pool = PgPoolOptions::new()
        .max_connections(70)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();

    let (_drop_guard, client, http_port, mut mailcrab_rx, smtp_port) = setup(pool).await;

    // Organization 2 has a rate limit of 0 that should be reset automatically to 120
    let (org_id, project_id, stream_id) = TestStreams::Org2Project1Stream1.get_stringified_ids();

    let john_cred = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{org_id}/projects/{project_id}/streams/{stream_id}/smtp_credentials"
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

    let mut join_set = JoinSet::new();

    join_set.spawn(async move {
        for i in 1.. {
            select! {
                Ok(recv) = mailcrab_rx.recv() => {
                    assert_eq!(recv.envelope_from.as_str(), "john@test-org-2-project-1.com");
                    assert_eq!(recv.envelope_recipients.len(), 1);
                    assert_eq!(recv.envelope_recipients[0].as_str(), "eddy@test-org-1-project-1.com");
                    if i > 120 {
                        panic!("went over rate limit")
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    if i < 120 {
                        panic!("timed out receiving {i}th email")
                    }
                    return
                },
            }
        }
    });

    // Spawn 10 tasks to send 15 messages each to remails, only 120 of these should be accepted
    for i in 0..10 {
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

        join_set.spawn(async move {
            for j in 1..=15 {
                let message = MessageBuilder::new()
                    .from(("John", "john@test-org-2-project-1.com"))
                    .to(vec![("Eddy", "eddy@test-org-1-project-1.com")])
                    .subject("TPS reports")
                    .text_body(format!(
                        "Have you finished the TPS reports yet? This is the {}th reminder!!!",
                        i * 2 + j
                    ));

                match john_smtp_client.send(message).await {
                    Ok(_) => (),
                    Err(mail_send::Error::UnexpectedReply(response)) => {
                        assert_eq!(response.code, 450);
                        assert_eq!(response.esc, [4, 3, 2]);
                        assert_eq!(response.message, "Sent too many messages, try again later");
                        return; // early exit because connection has been terminated
                    }
                    Err(e) => panic!("Error sending mail {e}"),
                }
            }
            let _ = john_smtp_client.quit().await;
        });
    }

    join_set.join_all().await;
}
