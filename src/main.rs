use coordinator::{node::Node, NodeCoordinator};
use iface::Interface;
use tunnels::{tls::TLSTunnel, tunnel::TunnelType};
use types::KB;

use std::{sync::Arc, time::Duration};

mod config;
mod coordinator;
mod iface;
mod tunnels;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let conf = config::from_file("config.toml")?;

    let tun = Interface::new_tun(conf.iface)?;
    let (tun_tx, mut tun_rx) = tun.forward().await?;

    let nodes = vec![Arc::new(Node {
        id: conf.nodes[0].id.clone(),
        addr: conf.nodes[0].addr.parse().unwrap(),
        tunnel_type: TunnelType::TLS,
        tunnel: TLSTunnel::new()
            .set_addr(conf.nodes[0].addr.parse().unwrap())
            .set_session_ttl(Duration::from_secs(10))
            .set_keepwarm(true)
            .set_prevent_tot(false)
            .set_ca(conf.nodes[0].ca.as_ref().unwrap().clone())
            .set_sni(conf.nodes[0].sni.as_ref().unwrap().clone()),
        max_fragment_size: 16 * KB,
    })];

    let node_coord = Arc::new(NodeCoordinator::new(nodes));
    let (coord_tx, mut coord_rx) = node_coord.forward();

    tokio::spawn(async move {
        while let Some(payload) = tun_rx.recv().await {
            let _ = coord_tx.send(payload).await;
        }
    });

    tokio::spawn(async move {
        while let Some(payload) = coord_rx.recv().await {
            println!("{}", payload.len());
            let _ = tun_tx.send(payload).await;
        }
    });

    tokio::time::sleep(Duration::from_secs(100)).await;
    Ok(())
}
