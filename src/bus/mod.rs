pub mod client;
pub mod server;

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};

    use futures::StreamExt;
    use rand::Rng;
    use tokio::sync::broadcast;
    use uuid::Uuid;

    use crate::bus::{
        client::{BusClient, BusMessage},
        server::Bus,
    };

    #[tokio::test]
    async fn multiple_listeners() {
        let bus_port = Bus::spawn_random_port().await;
        let client = BusClient::new(bus_port, "localhost".to_owned()).unwrap();

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
        client.send(&message).await.unwrap_err(); // should error
        client.try_send(&message).await; // ignores error

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
