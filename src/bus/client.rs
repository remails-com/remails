use async_stream::stream;
use serde::{Deserialize, Serialize};

use crate::models::{MessageId, MessageStatus};

use futures::{Stream, StreamExt};
pub type BusStream<'a> = std::pin::Pin<Box<dyn Stream<Item = BusMessage> + Send + 'a>>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum BusMessage {
    EmailReadyToSend(MessageId),
    EmailDeliveryAttempted(MessageId, MessageStatus),
}

#[derive(Clone)]
pub struct BusClient {
    client: reqwest::Client,
    port: u16,
    domain_name: String,
}

impl BusClient {
    pub fn new(port: u16, domain_name: String) -> Result<Self, reqwest::Error> {
        Ok(BusClient {
            client: reqwest::ClientBuilder::new().build()?,
            port,
            domain_name,
        })
    }

    pub fn new_from_env_var() -> Result<Self, reqwest::Error> {
        let port = std::env::var("MESSAGE_BUS_PORT")
            .unwrap_or("4000".to_owned())
            .parse()
            .expect("MESSAGE_BUS_PORT must be a u16");
        let domain_name = std::env::var("MESSAGE_BUS_FQDN").unwrap_or("localhost".to_owned());

        BusClient::new(port, domain_name)
    }

    pub async fn send(&self, message: &BusMessage) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("http://{}:{}/post", self.domain_name, self.port))
            .json(&message)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn receive(&'_ self) -> Result<BusStream<'_>, tokio_tungstenite::tungstenite::Error> {
        let ws_address = format!("ws://{}:{}/listen", self.domain_name, self.port);
        let (ws_stream, _) =
            tokio_tungstenite::connect_async_with_config(ws_address, None, false).await?;

        let (_, mut receiver) = ws_stream.split();

        Ok(Box::pin(stream! {
            while let Some(Ok(msg)) = receiver.next().await {
                match msg {
                    tokio_tungstenite::tungstenite::Message::Text(m) => {
                        match serde_json::from_str(&m) {
                            Ok(m) => yield m,
                            Err(e) => tracing::error!("could not deserialize WS message: {e:?}"),
                        }
                    }
                    m => {
                        tracing::error!("received invalid WS message: {m:?}");
                    }
                };
            }
        }))
    }

    pub fn receive_auto_reconnect(&'_ self, timeout: std::time::Duration) -> BusStream<'_> {
        Box::pin(stream! {
            loop {
                match self.receive().await {
                    Ok(mut stream) => while let Some(message) = stream.next().await {
                        yield message;
                    },
                    Err(e) => {
                        tracing::error!("reconnecting in {timeout:?} seconds... {e:?}");
                        tokio::time::sleep(timeout).await;
                    },
                }
            }
        })
    }
}
