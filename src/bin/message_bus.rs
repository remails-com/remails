use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::Message},
    response::IntoResponse,
    routing::{get, post},
};
use http::StatusCode;
use remails::init_tracing;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::sync::broadcast;

#[derive(Clone)]
struct BusState {
    message_tx: broadcast::Sender<String>,
}

struct Bus {
    router: Router,
    socket: SocketAddrV4,
}

impl Bus {
    fn new(socket: SocketAddrV4, message_tx: broadcast::Sender<String>) -> Self {
        let router = Router::new()
            .route("/listen", get(ws_handler))
            .route("/post", post(new_message))
            .with_state(BusState { message_tx });

        Bus { router, socket }
    }

    async fn serve(self) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(self.socket).await?;

        tracing::info!(
            "message bus {} serving on port {}",
            self.socket.ip(),
            self.socket.port()
        );

        axum::serve(listener, self.router).await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

    let port = std::env::var("MESSAGE_BUS_PORT")
        .unwrap_or("4000".to_owned())
        .parse()
        .expect("MESSAGE_BUS_PORT must be a u16");

    let socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
    let (tx, _rx) = broadcast::channel::<String>(100);
    let bus = Bus::new(socket, tx);

    bus.serve().await
}

async fn new_message(State(state): State<BusState>, body: String) -> impl IntoResponse {
    tracing::info!("new message: {body}");
    match state.message_tx.send(body) {
        Ok(n) => {
            tracing::trace!("sent message to {n} listeners");
            (StatusCode::ACCEPTED, format!("{n}"))
        }
        Err(e) => {
            tracing::error!("error sending message: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))
        }
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<BusState>) -> impl IntoResponse {
    let mut messages_rx = state.message_tx.subscribe();

    ws.on_upgrade(|mut socket| async move {
        while let Ok(message) = messages_rx.recv().await {
            if let Err(e) = socket.send(Message::Text(message.into())).await {
                tracing::error!("Error sending WS message: {e}");
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use rand::Rng;
    use remails::messaging::{BusClient, BusMessage};
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn multiple_listeners() {
        let mut rng = rand::rng();
        let port = rng.random_range(10_000..30_000);

        // spawn message bus
        let socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
        let (tx, _rx) = broadcast::channel::<String>(100);
        let bus = Bus::new(socket, tx);
        tokio::spawn(bus.serve());

        let client = BusClient::new(port, "localhost".to_owned()).unwrap();

        // two listeners
        let mut stream1 = client.receive().await.unwrap();
        let mut stream2 = client.receive().await.unwrap();

        // send a message
        let message = BusMessage::EmailReadyToSend(Uuid::new_v4().into());
        client.send(&message).await.unwrap();

        // both listeners should receive the message
        let received = stream1.next().await.unwrap();
        assert_eq!(received, message);

        let received = stream2.next().await.unwrap();
        assert_eq!(received, message);
    }

    #[tokio::test]
    async fn auto_reconnect() {
        let mut rng = rand::rng();
        let port = rng.random_range(10_000..30_000);

        // start receiving, even though message bus is offline
        let client = BusClient::new(port, "localhost".to_owned()).unwrap();
        let mut stream = client.receive_auto_reconnect(std::time::Duration::from_millis(500));

        let message = BusMessage::EmailReadyToSend(Uuid::new_v4().into());

        let mut listen = async || {
            let received = stream.next().await.unwrap();
            assert_eq!(received, message);
        };

        let host_and_post = async || {
            // spawn message bus
            let socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
            let (tx, _rx) = broadcast::channel::<String>(100);
            let bus = Bus::new(socket, tx);
            tokio::spawn(bus.serve());

            // send message after listener has had time to reconnect
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            client.send(&message).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            panic!("timeout!");
        };

        tokio::select! {
            _ = listen() => (),
            _ = host_and_post() => (),
        }
    }
}
