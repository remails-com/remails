use async_stream::stream;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

use crate::models::{MessageId, MessageStatus};

use futures::{Stream, StreamExt};
use tracing::log::trace;

pub type BusStream<'a> = std::pin::Pin<Box<dyn Stream<Item = BusMessage> + Send + 'a>>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum BusMessage {
    /// Message is ready to be sent from [`IpAddr`]
    EmailReadyToSend(MessageId, IpAddr),
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

    /// Initialize a `BusClient` by getting the port and domain name from the environment variables
    /// `MESSAGE_BUS_PORT` and `MESSAGE_BUS_FQDN`
    ///
    /// Will panic if `MESSAGE_BUS_PORT` is set to anything that cannot be parsed as a `u16`
    pub fn new_from_env_var() -> Result<Self, reqwest::Error> {
        let port = std::env::var("MESSAGE_BUS_PORT")
            .unwrap_or("4000".to_owned())
            .parse()
            .expect("MESSAGE_BUS_PORT must be a u16");
        let domain_name = std::env::var("MESSAGE_BUS_FQDN").unwrap_or("localhost".to_owned());

        BusClient::new(port, domain_name)
    }

    /// Send a message to the message bus
    pub async fn send(&self, message: &BusMessage) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("http://{}:{}/post", self.domain_name, self.port))
            .json(&message)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Send a message to the message bus, in case of error just log the error and ignore the result
    pub async fn try_send(&self, message: &BusMessage) {
        let _ = self
            .send(message)
            .await
            .inspect_err(|e| tracing::error!("Error sending bus message: {e}"));
    }

    /// Receive messages from the message bus
    ///
    /// Stream will end when WebSocket disconnects
    pub async fn receive(&'_ self) -> Result<BusStream<'_>, tokio_tungstenite::tungstenite::Error> {
        let ws_address = format!("ws://{}:{}/listen", self.domain_name, self.port);
        trace!("Connecting to message bus at {ws_address}");
        let (ws_stream, _) = tokio_tungstenite::connect_async(ws_address).await?;

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

    /// Receive messages from the message bus, while automatically reconnecting the WebSocket
    /// (with some timeout delay between connection attempts)
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

    /// For testing: wait for a certain number of messages to be ready and attempted by listening
    /// to the message bus
    ///
    /// Times out if no bus message is received for 5 seconds
    #[cfg(test)]
    pub async fn wait_for_attempt(messages_to_attempt: u32, stream: &mut BusStream<'_>) {
        let mut ready = 0;
        let mut attempted = 0;
        while ready < messages_to_attempt || attempted < messages_to_attempt {
            match tokio::time::timeout(tokio::time::Duration::from_secs(5), stream.next())
                .await
                .unwrap()
                .unwrap()
            {
                BusMessage::EmailReadyToSend(_, _) => ready += 1,
                BusMessage::EmailDeliveryAttempted(_, _) => attempted += 1,
            }
        }
    }
}
