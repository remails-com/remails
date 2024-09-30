use std::net::{Ipv4Addr, SocketAddrV4};
use message::Message;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;
use tokio::sync::mpsc;

mod message;
mod connection;
mod server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("remails=trace".parse().unwrap()),
        )
        .try_init()
        .unwrap();

    let (queue_sender, _qeueue_receiver) = mpsc::channel::<Message>(100);

    let socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1025);
    let shutdown = CancellationToken::new();
    let server =
        server::SmtServer::new(socket.into(), "cert.pem".into(), "key.pem".into(), queue_sender, shutdown);

    server.serve().await.unwrap();
}

#[cfg(test)]
mod test {
    use super::*;
    use mail_send::mail_builder::MessageBuilder;
    use mail_send::SmtpClientBuilder;
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn test_smtp() {
        let socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 1025);
        let shutdown = CancellationToken::new();
        let (queue_sender, mut receiver) = mpsc::channel::<Message>(100);
        let server = server::SmtServer::new(
            socket.into(),
            "cert.pem".into(),
            "key.pem".into(),
            queue_sender,
            shutdown.clone(),
        );

        let server_handle = tokio::spawn(async move {
            server.serve().await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let message = MessageBuilder::new()
            .from(("John Doe", "john@example.com"))
            .to(vec![
                ("Jane Doe", "jane@example.com"),
                ("James Smith", "james@test.com"),
            ])
            .subject("Hi!")
            .html_body("<h1>Hello, world!</h1>")
            .text_body("Hello world!");

        SmtpClientBuilder::new("localhost", 1025)
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
        assert_eq!(received_message.get_from(), "john@example.com");
    }
}
