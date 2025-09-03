mod config;
mod coordinator;

use std::sync::Arc;
use std::time::Duration;

use coordinator::{NodeCoordinator, node::Node};
use device::Device;
use device::config::DeviceConfig;
use tunnels::{TunnelType, outgoing};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let conf = config::from_file("../configs/client.toml")?;

    let device = new_network_device(conf.device)?;
    let (tun_tx, mut tun_rx) = device.forward().await?;

    let nodes = vec![Arc::new(Node {
        id: conf.nodes[2].id.clone(),
        addr: conf.nodes[2].addr.parse().unwrap(),
        tunnel_type: TunnelType::Tls,
        tunnel: outgoing::tls::TLSTunnel::new()
            .set_addr(conf.nodes[2].addr.parse().unwrap())
            .set_session_ttl(Duration::from_secs(10))
            .set_keepwarm(true)
            .set_ca(conf.nodes[2].ca.as_ref().unwrap().clone())
            .set_sni(conf.nodes[2].sni.as_ref().unwrap().clone()),
        max_fragment_size: 16 * 1024,
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
            let _ = tun_tx.send(payload).await;
        }
    });

    tokio::time::sleep(Duration::from_secs(10000)).await;
    Ok(())
}

fn new_network_device(conf: config::DeviceConfig) -> anyhow::Result<Device> {
    Device::new_tun(DeviceConfig {
        name: conf.name,
        mtu: conf.mtu,
        addr: conf.addr.parse().unwrap(),
        mask: conf.mask,
        disable_on_exit: conf.disable_on_exit,
    })
}
