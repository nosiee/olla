use std::sync::Arc;

use device::{Device, config::DeviceConfig};
use tunnels::{AsyncIncomingTunnel, TunnelType, incoming::udp::UDPTunnel};

mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let conf = config::from_file("configs/node.toml")?;
    let device = new_network_device(conf.device)?;
    let (tun_tx, mut tun_rx) = device.forward().await?;

    for tunnel in conf.tunnels {
        match tunnel.tunnel_type.into() {
            TunnelType::Tls => {
                todo!();
            }
            TunnelType::Udp => {
                let udptun = Arc::new(UDPTunnel::new(tunnel.addr.parse().unwrap()).await);
                let _ = udptun.clone().forward(tun_tx.clone()).await;

                while let Some(payload) = tun_rx.recv().await {
                    // FIXME(nosiee): ??????????????
                    udptun.write("123.123.123.123:1234".into(), &payload).await.unwrap();
                }
            }
            TunnelType::Tcp => {
                todo!();
            }
            TunnelType::Rtmp => {
                todo!();
            }
            TunnelType::Unknown => {
                todo!();
            }
        }
    }

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
