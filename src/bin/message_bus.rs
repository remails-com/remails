use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::Message},
    response::IntoResponse,
    routing::{get, post},
};
use http::StatusCode;
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::sync::broadcast;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

type BusMessage = String;

#[derive(Clone)]
struct BusState {
    message_tx: broadcast::Sender<BusMessage>,
}

struct Bus {
    router: Router,
    socket: SocketAddrV4,
}

impl Bus {
    fn new(socket: SocketAddrV4, message_tx: broadcast::Sender<BusMessage>) -> Self {
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
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=trace,tower_http=debug,axum=trace,info",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 3700);
    let (tx, _rx) = broadcast::channel::<BusMessage>(100);
    let bus = Bus::new(socket, tx);

    bus.serve().await
}

async fn new_message(State(state): State<BusState>, body: BusMessage) -> impl IntoResponse {
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
