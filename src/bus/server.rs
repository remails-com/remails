use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::Message},
    response::IntoResponse,
    routing::{get, post},
};
use http::StatusCode;
use humansize::ToF64;
use std::net::SocketAddrV4;
use tokio::sync::broadcast;
use tracing::{error, trace, warn};

pub const CAPACITY: usize = 128;

#[derive(Clone)]
struct BusState {
    message_tx: broadcast::Sender<String>,
}

pub struct Bus {
    router: Router,
    socket: SocketAddrV4,
}

impl Bus {
    pub fn new(socket: SocketAddrV4, message_tx: broadcast::Sender<String>) -> Self {
        let router = Router::new()
            .route("/listen", get(ws_handler))
            .route("/post", post(new_message))
            .route("/healthy", get(healthy))
            .with_state(BusState { message_tx });

        Bus { router, socket }
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(self.socket).await?;

        tracing::info!(
            "message bus {} serving on port {}",
            self.socket.ip(),
            self.socket.port()
        );

        axum::serve(listener, self.router).await?;

        Ok(())
    }

    #[cfg(test)]
    pub async fn spawn_random_port() -> u16 {
        let mut rng = rand::rng();

        let bus_port = rand::RngExt::random_range(&mut rng, 10_000..30_000);
        let bus_socket = SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), bus_port);

        let tx = broadcast::Sender::<String>::new(CAPACITY);
        let bus = Bus::new(bus_socket, tx);
        tokio::spawn(async { bus.serve().await });

        bus_port
    }
}

async fn new_message(State(state): State<BusState>, body: String) -> impl IntoResponse {
    tracing::info!("new message: {body}");
    match state.message_tx.send(body) {
        Ok(n) => {
            trace!("sent message to {n} listeners");
            (StatusCode::ACCEPTED, format!("{n}"))
        }
        Err(e) => {
            error!("error sending message (probably no active subscribers): {e}");
            (StatusCode::ACCEPTED, "0".to_string())
        }
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<BusState>) -> impl IntoResponse {
    let mut messages_rx = state.message_tx.subscribe();

    ws.on_upgrade(|mut socket| async move {
        while let Ok(message) = messages_rx.recv().await {
            if let Err(e) = socket.send(Message::Text(message.into())).await {
                error!("Error sending WS message: {e}");
            }
        }
    })
}

async fn healthy(State(state): State<BusState>) -> StatusCode {
    trace!("called /healthy");

    if state.message_tx.len().to_f64() > CAPACITY.to_f64() * 0.7 {
        warn!(
            receiver_count = state.message_tx.receiver_count(),
            "Message bus has a huge backlog ({} of max {})",
            state.message_tx.len(),
            CAPACITY
        )
    }

    StatusCode::OK
}
