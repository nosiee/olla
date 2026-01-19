use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use super::config;
use super::coordinator::node::{Node, NodeCoordinator};
use super::device::{Device, config::DeviceConfig};
use super::tunnels::{header::HEADER_SIZE, outgoing};

pub async fn run(path: PathBuf) -> anyhow::Result<()> {
    let config = config::from_file(path)?;
    let primary_node = config
        .nodes
        .iter()
        .find(|n| n.primary.unwrap_or_default())
        .expect("the primary node must be set");

    let nodes = create_nodes(&config.nodes, config.device.mtu as usize, primary_node.addr.parse().unwrap());
    let device = new_network_device(&config.device)?;
    let (tun_tx, tun_rx) = device.forward().await?;

    let node_coord = Arc::new(NodeCoordinator::new(nodes));
    let (nc_tx, nc_rx) = node_coord.forward();

    tokio::spawn(async move {
        while let Ok(payload) = tun_rx.recv().await {
            let _ = nc_tx.send(payload).await;
        }
    });

    while let Ok(payload) = nc_rx.recv().await {
        tun_tx.send(payload).await.unwrap();
    }

    Ok(())
}

fn create_nodes(nc: &Vec<config::NodeConfig>, mtu: usize, primary_node: SocketAddr) -> Vec<Arc<Node>> {
    let mut nodes = Vec::with_capacity(nc.len());

    for node in nc {
        let node = Node {
            id: node.id.clone(),
            addr: node.addr.parse().unwrap(),
            tunnel: outgoing::OutgoingTunnel::new()
                .set_addr(node.addr.parse().unwrap())
                .set_primary_node(primary_node),
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
