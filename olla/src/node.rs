use async_channel::{Receiver, Sender};
use socket2::SockAddr;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::error;

use super::config;
use super::coordinator::{node::Node, packet::PacketCoordinator, packet::PacketCoordinatorMessage};
use super::device::{self, Device, config::DeviceConfig};
use super::tunnels::{header::HEADER_SIZE, incoming, outgoing};

pub async fn run(path: PathBuf) -> anyhow::Result<()> {
    let config = config::from_file(path)?;

    let nodes = create_nodes(&config.nodes, config.device.mtu as usize);
    let device = new_network_device(&config.device)?;

    let machine_addr = device::util::get_device_ipv4("eth0").unwrap();
    let packet_coordinator = Arc::new(PacketCoordinator::new(machine_addr, nodes));

    let (tun_tx, tun_rx) = device.forward().await?;
    let (pc_tx, pc_rx) = packet_coordinator.forward(tun_tx, tun_rx);

    run_tunnel(config.tunnel.unwrap(), pc_tx, pc_rx).await
}

fn create_nodes(nc: &Vec<config::NodeConfig>, mtu: usize) -> Vec<Arc<Node>> {
    let mut nodes = Vec::with_capacity(nc.len());

    for node in nc {
        let node = Node {
            id: node.id.clone(),
            addr: node.addr.parse().unwrap(),
            tunnel: outgoing::OutgoingTunnel::new()
                .set_addr(node.addr.parse().unwrap())
                .set_keepalive(node.keepalive.unwrap_or_default()),
            max_fragment_size: mtu + HEADER_SIZE,
        };

        nodes.push(Arc::new(node));
    }

    nodes
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

async fn run_tunnel(
    tunnel: config::TunnelConfig,
    tx: Sender<PacketCoordinatorMessage>,
    rx: Receiver<PacketCoordinatorMessage>,
) -> anyhow::Result<()> {
    let addr: SocketAddr = tunnel.addr.parse().unwrap();
    let mut incomingtun = incoming::IncomingTunnel::new(SockAddr::from(addr));
    let _ = incomingtun.forward(tx).await;

    while let Ok((peer, payload)) = rx.recv().await {
        if let Err(err) = incomingtun.write(peer, &payload).await {
            error!("failed to write payload: {:?}", err);
        }
    }

    Ok(())
}
