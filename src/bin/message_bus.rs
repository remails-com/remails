use remails::{
    bus::server::{Bus, CAPACITY},
    init_tracing,
};
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::sync::broadcast;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    init_tracing();

    let port = std::env::var("MESSAGE_BUS_PORT")
        .unwrap_or("4000".to_owned())
        .parse()
        .expect("MESSAGE_BUS_PORT must be a u16");

    let socket = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
    let tx = broadcast::Sender::<String>::new(CAPACITY);
    let bus = Bus::new(socket, tx);

    bus.serve().await
}
