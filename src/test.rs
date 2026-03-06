use crate::{
    Environment,
    bus::{client::BusClient, server::Bus},
    handler::{HandlerConfig, RetryConfig, dns::DnsResolver},
    models::{
        ApiKey, ApiMessage, ApiMessageMetadata, CreatedApiKeyWithPassword, MessageStatus,
        OrgBlockStatus, OrganizationId, Project, ProjectId, SmtpCredential, SmtpCredentialResponse,
    },
    run_api_server, run_mta,
    smtp::SmtpConfig,
};
use http::{HeaderMap, StatusCode, header, header::CONTENT_TYPE};
use mail_send::{SmtpClientBuilder, mail_builder::MessageBuilder};
use mailcrab::TestMailServerHandle;
use rand::RngExt;
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{select, task::JoinSet};

pub fn random_port() -> u16 {
    let mut rng = rand::rng();

    rng.random_range(10_000..30_000)
}

/// Streams used for testing, as configured in the fixtures
#[allow(dead_code)]
pub enum TestProjects {
    Org1Project1,
    Org1Project2,
    Org2Project1,
}

impl TestProjects {
    pub fn project_id(&self) -> ProjectId {
        match self {
            TestProjects::Org1Project1 => "3ba14adf-4de1-4fb6-8c20-50cc2ded5462".parse().unwrap(),
            TestProjects::Org1Project2 => "da12d059-d86e-4ac6-803d-d013045f68ff".parse().unwrap(),
            TestProjects::Org2Project1 => "70ded685-8633-46ef-9062-d9fbad24ae95".parse().unwrap(),
        }
    }

    pub fn org_id(&self) -> OrganizationId {
        match self {
            TestProjects::Org1Project1 | TestProjects::Org1Project2 => {
                "44729d9f-a7dc-4226-b412-36a7537f5176".parse().unwrap()
            }
            TestProjects::Org2Project1 => "5d55aec5-136a-407c-952f-5348d4398204".parse().unwrap(),
        }
    }

    pub fn get_ids(&self) -> (OrganizationId, ProjectId) {
        (self.org_id(), self.project_id())
    }

    pub fn get_stringified_ids(&self) -> (String, String) {
        (self.org_id().to_string(), self.project_id().to_string())
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
        cert_file: "dev-secrets/cert.pem".into(),
        key_file: "dev-secrets/key.pem".into(),
        environment: Default::default(),
        retry: retry_config.clone(),
    };

    let handler_config = HandlerConfig {
        domain: "test".to_owned(),
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
        bus_client.clone(),
        token.clone(),
    )
    .await;
    run_api_server(pool, bus_client, http_socket, token.clone(), false, false).await;

    let _drop_guard = token.drop_guard();

    (_drop_guard, client, http_port, mailcrab_rx, smtp_port)
}

#[sqlx::test(fixtures(
    "organizations",
    "api_users",
    "projects",
    "org_domains",
    "proj_domains",
    "k8s_nodes"
))]
async fn integration_test(pool: PgPool) {
    let (_drop_guard, client, http_port, mut mailcrab_rx, smtp_port) = setup(pool).await;

    // create John's SMTP credential
    let (jorg, jproj) = TestProjects::Org1Project1.get_stringified_ids();
    let john_cred = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{jorg}/projects/{jproj}/smtp_credentials"
        ))
        .header("X-Test-Login", &jorg)
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

    // check Johns's SMTP credential exists
    let credentials: Vec<SmtpCredential> = client
        .get(format!(
            "http://localhost:{http_port}/api/organizations/{jorg}/projects/{jproj}/smtp_credentials"
        ))
        .header("X-Test-Login", &jorg)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(credentials.len(), 1);

    // create Eddy's REST API credential
    let (eorg, eproj) = TestProjects::Org2Project1.get_stringified_ids();
    let eddy_cred = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{eorg}/api_keys"
        ))
        .header("X-Test-Login", &eorg)
        .json(&json!({
            "role": "maintainer", // read-write
            "description": "Eddy test credential"
        }))
        .send()
        .await
        .unwrap()
        .json::<CreatedApiKeyWithPassword>()
        .await
        .unwrap();

    // check Eddy's REST API credential exists
    let credentials: Vec<ApiKey> = client
        .get(format!(
            "http://localhost:{http_port}/api/organizations/{eorg}/api_keys"
        ))
        .header("X-Test-Login", &eorg)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(credentials.len(), 1);

    // John sends some message via SMTP
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
            ))
            .message_id(format!("tps-{i}@test-org-1-project-1.com"));
        john_smtp_client.send(message).await.unwrap();
    }

    // check messages were received
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

    // Eddy sends a message via the Remails REST API
    let message: ApiMessageMetadata = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{eorg}/projects/{eproj}/emails"
        ))
        .basic_auth(eddy_cred.id(), Some(eddy_cred.password()))
        .json(&json!({
            "from": {"name": "Eddy", "address": "eddy@test-org-2-project-1.com"},
            "to": {"name": "John", "address": "john@test-org-1-project-1.com"},
            "subject": "Re: TPS reports",
            "text_body": "Ah! Yeah. It's just we're putting new coversheets on all the TPS reports before they go out now.
        So if you could go ahead and try to remember to do that from now on, that'd be great. All right!",
            "in_reply_to": "tps-10@test-org-1-project-1.com"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(message.smtp_credential_id, None);
    assert_eq!(message.api_key_id, Some(*eddy_cred.id()));

    // check message was received
    select! {
        Ok(recv) = mailcrab_rx.recv() => {
            assert_eq!(recv.envelope_from.as_str(), "eddy@test-org-2-project-1.com");
            assert_eq!(recv.envelope_recipients.len(), 1);
            assert_eq!(recv.envelope_recipients[0].as_str(), "john@test-org-1-project-1.com");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => panic!("timed out receiving email"),
    }

    // check John's sent messages
    let messages: Vec<ApiMessageMetadata> = client
        .get(format!(
            "http://localhost:{http_port}/api/organizations/{jorg}/emails"
        ))
        .header("X-Test-Login", &jorg)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(messages.len(), 10);

    // cannot check someone else's messages
    let status = client
        .get(format!(
            "http://localhost:{http_port}/api/organizations/{jorg}/emails"
        ))
        .header("X-Test-Login", "00000000-0000-4000-0000-000000000000") // non-existent organization
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(status, StatusCode::FORBIDDEN);

    // set John's project to no plaintext_fallback
    let project: Project = client
        .put(format!(
            "http://localhost:{http_port}/api/organizations/{jorg}/projects/{jproj}"
        ))
        .header("X-Test-Login", &jorg)
        .json(&json!({
            "name": "Project 1 Organization 1",
            "retention_period_days": 1,
            "plaintext_fallback": false
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!project.plaintext_fallback);
    tokio::time::sleep(Duration::from_secs(1)).await;

    // John sends another message
    let message = MessageBuilder::new()
        .from(("John", "john@test-org-1-project-1.com"))
        .to(vec![("Eddy", "eddy@test-org-2-project-1.com")])
        .subject("TPS reports")
        .text_body("Have you finished the TPS reports yet? This is the 11th reminder!!!")
        .message_id("tps-11@test-org-1-project-1.com");
    john_smtp_client.send(message).await.unwrap();

    // Retrieve messages
    let messages: Vec<ApiMessageMetadata> = client
        .get(format!(
            "http://localhost:{http_port}/api/organizations/{jorg}/emails?limit=20"
        ))
        .header("X-Test-Login", &jorg)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(messages.len(), 11);
    let message_id = messages
        .into_iter()
        .find(|m| m.message_id_header == "tps-11@test-org-1-project-1.com")
        .unwrap()
        .id;

    // Status should become Failed because we don't use TLS for mailcrab
    let get_message = async || {
        client
            .get(format!(
                "http://localhost:{http_port}/api/organizations/{jorg}/emails/{}",
                message_id
            ))
            .header("X-Test-Login", &jorg)
            .send()
            .await
            .unwrap()
            .json::<ApiMessage>()
            .await
            .unwrap()
    };
    while matches!(
        get_message().await.status(),
        MessageStatus::Processing | MessageStatus::Accepted
    ) {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    assert_eq!(*get_message().await.status(), MessageStatus::Failed);

    // super admin blocks John's organization from sending emails
    let status = client
        .put(format!(
            "http://localhost:{http_port}/api/organizations/{jorg}/admin"
        ))
        .header("X-Test-Login-ID", "deadbeef-4e43-4a66-bbb9-fbcd4a933a34") // super admin
        .json(&OrgBlockStatus::NoSending)
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(status, StatusCode::OK);

    // John can still send emails
    let message = MessageBuilder::new()
        .from(("John2", "john2@test-org-1-project-1.com"))
        .to(vec![("Eddy", "eddy@test-org-2-project-1.com")])
        .subject("Hello?!1")
        .text_body("Is this working?")
        .message_id("hello@test-org-1-project-1.com");
    john_smtp_client.send(message).await.unwrap();

    // check John's sent messages
    let messages: Vec<ApiMessageMetadata> = client
        .get(format!(
            "http://localhost:{http_port}/api/organizations/{jorg}/emails?limit=20"
        ))
        .header("X-Test-Login", &jorg)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(messages.len(), 12);
    let message = messages
        .into_iter()
        .find(|m| m.from_email.as_str() == "john2@test-org-1-project-1.com")
        .unwrap();
    // the message is left on Processing because it does not get send to a handler
    assert_eq!(message.status, MessageStatus::Processing);

    // super admin blocks John's organization from sending and receiving emails
    let status = client
        .put(format!(
            "http://localhost:{http_port}/api/organizations/{jorg}/admin"
        ))
        .header("X-Test-Login-ID", "deadbeef-4e43-4a66-bbb9-fbcd4a933a34") // super admin
        .json(&OrgBlockStatus::NoSendingOrReceiving)
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(status, StatusCode::OK);

    // John can no longer send emails via the SMTP in-bound server
    let message = MessageBuilder::new()
        .from(("John2", "john2@test-org-1-project-1.com"))
        .to(vec![("Eddy", "eddy@test-org-2-project-1.com")])
        .subject("Hello??!1")
        .text_body("Is this working??")
        .message_id("hello2@test-org-1-project-1.com");
    let err = john_smtp_client.send(message).await.unwrap_err();
    assert!(matches!(err, mail_send::Error::UnexpectedReply(_)));

    // super admin blocks Eddy's organization from sending and receiving emails
    let status = client
        .put(format!(
            "http://localhost:{http_port}/api/organizations/{eorg}/admin"
        ))
        .header("X-Test-Login-ID", "deadbeef-4e43-4a66-bbb9-fbcd4a933a34") // super admin
        .json(&OrgBlockStatus::NoSendingOrReceiving)
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(status, StatusCode::OK);

    // Eddy can no longer send emails via the REST API
    let res = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{eorg}/projects/{eproj}/emails"
        ))
        .basic_auth(eddy_cred.id(), Some(eddy_cred.password()))
        .json(&json!({
            "from": {"name": "Eddy", "address": "eddy@test-org-2-project-1.com"},
            "to": {"name": "John", "address": "john@test-org-1-project-1.com"},
            "subject": "Re: TPS reports",
            "text_body": "Ah! Yeah. It's just we're putting new coversheets on all the TPS reports before they go out now.
        So if you could go ahead and try to remember to do that from now on, that'd be great. All right!",
            "in_reply_to": "tps-10@test-org-1-project-1.com"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(fixtures(
    "organizations",
    "api_users",
    "projects",
    "org_domains",
    "proj_domains",
    "k8s_nodes"
))]
async fn quotas_count_atomically(pool: PgPool) {
    let pool = PgPoolOptions::new()
        .max_connections(70)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();

    let (_drop_guard, client, http_port, mut mailcrab_rx, smtp_port) = setup(pool.clone()).await;

    let (org_id, project_id) = TestProjects::Org1Project1.get_ids();

    sqlx::query!(
        r#"
        UPDATE organizations
        SET current_subscription = jsonb_set(current_subscription, '{product}', '"UNLIMITED"')
        WHERE id = $1;
        "#,
        *org_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let john_cred = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{org_id}/projects/{project_id}/smtp_credentials"
        ))
        .header("X-Test-Login", org_id.to_string())
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

    // Spawn 11 tasks to throw 100 messages each to remails
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
    "k8s_nodes"
))]
async fn rate_limit_count_atomically(pool: PgPool) {
    tracing_subscriber::fmt::init();
    let pool = PgPoolOptions::new()
        .max_connections(70)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();

    let (_drop_guard, client, http_port, mut mailcrab_rx, smtp_port) = setup(pool).await;

    // Organization 2 has a rate limit of 0 that should be reset automatically to 20
    let (org_id, project_id) = TestProjects::Org2Project1.get_stringified_ids();

    let john_cred = client
        .post(format!(
            "http://localhost:{http_port}/api/organizations/{org_id}/projects/{project_id}/smtp_credentials"
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

    let mut test_start = Instant::now();

    join_set.spawn(async move {
        for i in 1.. {
            select! {
                Ok(recv) = mailcrab_rx.recv() => {
                    if i == 1 {
                        test_start = Instant::now();
                    }
                    assert_eq!(recv.envelope_from.as_str(), "john@test-org-2-project-1.com");
                    assert_eq!(recv.envelope_recipients.len(), 1);
                    assert_eq!(recv.envelope_recipients[0].as_str(), "eddy@test-org-1-project-1.com");
                    if i > 60 + test_start.elapsed().as_millis() / 500 + 3 {
                        panic!("went over rate limit, received {i} mails {}ms after test start", test_start.elapsed().as_millis())
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    if i < 60 {
                        panic!("timed out receiving {i}th email")
                    }
                    return
                },
            }
        }
    });

    let count_rate_limits = Arc::new(AtomicUsize::new(0));

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

        let count_rate_limits = count_rate_limits.clone();
        join_set.spawn(async move {
            tokio::time::sleep(Duration::from_secs(i)).await;
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
                        count_rate_limits.fetch_add(1, Ordering::Relaxed);
                        break;
                    }
                    Err(e) => panic!("Error sending mail {e}"),
                }
            }
            let _ = john_smtp_client.quit().await;
        });
    }

    join_set.join_all().await;

    assert!(
        count_rate_limits.load(Ordering::Relaxed) > 0,
        "Expected at least one rate limit error, got none"
    );
}
