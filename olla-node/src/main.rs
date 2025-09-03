use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::types::{Message, PACKETS_BUFFER_SIZE};
use iface::Interface;
use tunnels::tls::TLSTunnel;
use tunnels::tunnel::{AsyncTunnel, TunnelType};

mod config;
mod iface;
mod tunnels;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let conf = config::from_file("config_node.toml")?;
    let iface = Interface::new_tun(conf.iface).unwrap();
    let (tx, mut rx): (Sender<Message>, Receiver<Message>) = iface.forward().await.unwrap();

    for tunnel in conf.tunnels {
        match tunnel.tunnel_type.into() {
            TunnelType::Tls => {
                let tun = Arc::new(TLSTunnel::new(tunnel.addr.parse().unwrap(), tunnel.cert.unwrap(), tunnel.key.unwrap()));
                tun.clone().forward(tx.clone()).await.unwrap();

                while let Some(buf) = rx.recv().await {
                    tun.write("1".to_string(), buf).await.unwrap();
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}
