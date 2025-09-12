mod config;

use std::sync::Arc;

use coordinator::{node::Node, node::NodeCoordinator};
use device::Device;
use device::config::DeviceConfig;
use tunnels::{TunnelType, header::HEADER_SIZE, outgoing};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let conf = config::from_file("configs/client.toml")?;
    let pnode = conf
        .nodes
        .iter()
        .find(|n| n.primary.unwrap_or_default())
        .expect("the primary node must be set");

    let nodes = vec![
        Arc::new(Node {
            id: conf.nodes[0].id.clone(),
            addr: conf.nodes[0].addr.parse().unwrap(),
            tunnel_type: TunnelType::Udp,
            tunnel: outgoing::udp::UDPTunnel::new()
                .set_addr(conf.nodes[0].addr.parse().unwrap())
                .set_keepalive(conf.nodes[0].keepalive.unwrap_or_default())
                .set_primary_node(pnode.addr.parse().unwrap()),
            max_fragment_size: conf.device.mtu as usize + HEADER_SIZE,
        }),
        Arc::new(Node {
            id: conf.nodes[1].id.clone(),
            addr: conf.nodes[1].addr.parse().unwrap(),
            tunnel_type: TunnelType::Udp,
            tunnel: outgoing::udp::UDPTunnel::new()
                .set_addr(conf.nodes[1].addr.parse().unwrap())
                .set_keepalive(conf.nodes[1].keepalive.unwrap_or_default())
                .set_primary_node(pnode.addr.parse().unwrap()),
            max_fragment_size: conf.device.mtu as usize + HEADER_SIZE,
        }),
    ];

    let device = new_network_device(&conf.device)?;
    let (tun_tx, mut tun_rx) = device.forward().await?;

    let node_coord = Arc::new(NodeCoordinator::new(nodes));
    let (nc_tx, mut nc_rx) = node_coord.forward();

    tokio::spawn(async move {
        while let Ok(payload) = tun_rx.recv().await {
            let _ = nc_tx.send(payload).await;
        }
    });

    while let Some(payload) = nc_rx.recv().await {
        let _ = tun_tx.send(payload).unwrap();
    }

    Ok(())
}

fn new_network_device(conf: &config::DeviceConfig) -> anyhow::Result<Device> {
    Device::new_tun(DeviceConfig {
        name: conf.name.clone(),
        mtu: conf.mtu,
        addr: conf.addr.parse().unwrap(),
        mask: conf.mask.clone(),
        disable_on_exit: conf.disable_on_exit,
    })
}
