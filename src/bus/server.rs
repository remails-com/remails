use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::Message},
    response::IntoResponse,
    routing::{get, post},
};
use http::StatusCode;
use std::net::SocketAddrV4;
use tokio::sync::broadcast;

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
