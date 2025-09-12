mod config;

use coordinator::node::Node;
use coordinator::packet::PacketCoordinator;
use device::{Device, config::DeviceConfig};
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::error;
use tunnels::header::HEADER_SIZE;
use tunnels::{AsyncIncomingTunnel, TunnelType, incoming, outgoing};
use types::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let conf = config::from_file("configs/node.toml")?;
    let nodes = vec![Arc::new(Node {
        id: conf.nodes[0].id.clone(),
        addr: conf.nodes[0].addr.parse().unwrap(),
        tunnel_type: TunnelType::Udp,
        tunnel: outgoing::udp::UDPTunnel::new()
            .set_addr(conf.nodes[0].addr.parse().unwrap())
            .set_keepalive(conf.nodes[0].keepalive.unwrap_or_default()),
        max_fragment_size: conf.device.mtu as usize + HEADER_SIZE,
    })];

    let device = new_network_device(conf.device)?;
    let machine_addr = device::util::get_device_ipv4("eth0").unwrap();
    let packet_coordinator = Arc::new(PacketCoordinator::new(machine_addr, nodes));

    let (tun_tx, tun_rx) = device.forward().await?;
    let (pc_tx, pc_rx) = packet_coordinator.forward(tun_tx, tun_rx);
    let _ = init_tunnels(conf.tunnels, pc_tx, pc_rx).await;

    tokio::time::sleep(std::time::Duration::from_secs(10000)).await;
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

async fn init_tunnels(
    tunnels: Vec<config::TunnelConfig>,
    tx: Sender<PacketCoordinatorMessage>,
    rx: Receiver<PacketCoordinatorMessage>,
) -> anyhow::Result<()> {
    for tunnel in tunnels {
        match tunnel.tunnel_type.into() {
            TunnelType::Udp => {
                let udptun = Arc::new(incoming::udp::UDPTunnel::new(tunnel.addr.parse().unwrap()).await);
                let _ = udptun.clone().forward(tx.clone()).await;
                let mut rxbc = rx.resubscribe();

                tokio::spawn(async move {
                    while let Ok((_, peer, payload)) = rxbc.recv().await {
                        if let Err(err) = udptun.write(peer, &payload).await {
                            error!("failed to write payload: {:?}", err);
                        }
                    }
                });
            }
            _ => unimplemented!(),
        }
    }

    Ok(())
}
