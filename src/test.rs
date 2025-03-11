use crate::{message::Message, run, user::User};
use http::{HeaderMap, header, header::CONTENT_TYPE};
use mail_send::{mail_builder::MessageBuilder, SmtpClientBuilder};
use rand::Rng;
use serde_json::json;
use sqlx::PgPool;
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{span, Instrument, Level};
use tracing_test::traced_test;
use crate::smtp::smtp_server::SmtpServer;
use crate::user::UserRepository;

pub fn random_port() -> u16 {
    let mut rng = rand::rng();

    rng.random_range(10_000..30_000)
}

async fn setup_smtp_recv(pool: PgPool, shutdown: CancellationToken) -> mpsc::Receiver<Message> {
    // let smtp_port = random_port();
    let smtp_port = 1025;
    let user_repository = UserRepository::new(pool);

    let socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), smtp_port);
    let (queue_sender, receiver) = mpsc::channel::<Message>(100);
    let server = SmtpServer::new(
        socket,
        "cert.pem".into(),
        "key.pem".into(),
        user_repository,
        queue_sender,
        shutdown,
    );
    let span = span!(Level::TRACE, "SMTP receiver");
    let server_handle = tokio::spawn(async move {
        server.serve().await.unwrap();
    }.instrument(span));

    receiver
}

#[sqlx::test]
#[traced_test]
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
    let http_port = random_port();

    let smtp_socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), smtp_port);
    let http_socket = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), http_port);

    let shutdown = run(pool, smtp_socket, http_socket).await;

    let user1: User = client
        .post(format!("http://localhost:{}/users", http_port))
        .header("X-Test-Login", "admin")
        .json(&json!({
            "username": "john",
            "password": "p4ssw0rd",
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let user2: User = client
        .post(format!("http://localhost:{}/users", http_port))
        .header("X-Test-Login", "admin")
        .json(&json!({
            "username": "eddy",
            "password": "pass123",
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let users: Vec<User> = client
        .get(format!("http://localhost:{}/users", http_port))
        .header("X-Test-Login", "admin")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(users.len(), 2);

    let mut john_smtp_client = SmtpClientBuilder::new("localhost", smtp_port)
        .implicit_tls(true)
        .allow_invalid_certs()
        .credentials(("john", "p4ssw0rd"))
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
    }

    let message = MessageBuilder::new()
        .from(("Eddy", "eddy@example.com" ))
        .to(vec![
            ("John", "john@example.com"),
        ])
        .subject("Re: TPS reports")
        .text_body("Ah! Yeah. It's just we're putting new coversheets on all the TPS reports before they go out now.
        So if you could go ahead and try to remember to do that from now on, that'd be great. All right!");

    SmtpClientBuilder::new("localhost", smtp_port)
        .implicit_tls(true)
        .allow_invalid_certs()
        .credentials(("eddy", "pass123"))
        .connect()
        .await
        .unwrap()
        .send(message)
        .await
        .unwrap();

    // TODO make test more robust, without sleep
    sleep(Duration::from_secs(1)).await;

    let messages: Vec<Message> = client
        .get(format!("http://localhost:{}/messages", http_port))
        .header("X-Test-Login", user1.get_id().to_string())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 10);

    let messages: Vec<Message> = client
        .get(format!("http://localhost:{}/messages", http_port))
        .header("X-Test-Login", user2.get_id().to_string())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 1);

    let messages: Vec<Message> = client
        .get(format!("http://localhost:{}/messages", http_port))
        .header("X-Test-Login", "admin")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(messages.len(), 11);

    shutdown.cancel();
}
