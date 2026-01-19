use async_channel::{Receiver, Sender};
use bytes::BytesMut;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error};

use crate::device::{DEVICE_BUFFER_SIZE, Message};
use crate::tunnels::errors::{NO_PEER_FOUND, TunnelError};
use crate::tunnels::header::{self, HEADER_SIZE};

use super::node::Node;
use crate::device;

pub type PacketCoordinatorMessage = (String, Message);

#[derive(Debug)]
pub struct PacketCoordinator {
    coordination_table: RwLock<HashMap<String, String>>,
    nodes: Vec<Arc<Node>>,
    primary_nodes: RwLock<HashMap<String, ()>>,

    machine_addr: Ipv4Addr,
}

impl PacketCoordinator {
    pub fn new(machine_addr: Ipv4Addr, nodes: Vec<Arc<Node>>) -> Self {
        Self {
            coordination_table: RwLock::new(HashMap::new()),
            nodes,
            primary_nodes: RwLock::new(HashMap::new()),

            machine_addr,
        }
    }

    pub fn forward(
        self: Arc<Self>,
        tun_dev_tx: Sender<Message>,
        tun_dev_rx: Receiver<Message>,
    ) -> (Sender<PacketCoordinatorMessage>, Receiver<PacketCoordinatorMessage>) {
        let (itx, irx): (Sender<PacketCoordinatorMessage>, Receiver<PacketCoordinatorMessage>) = async_channel::bounded(DEVICE_BUFFER_SIZE);
        let (otx, orx): (Sender<PacketCoordinatorMessage>, Receiver<PacketCoordinatorMessage>) = async_channel::bounded(DEVICE_BUFFER_SIZE);

        let cpu_cores = match std::thread::available_parallelism() {
            Ok(n) => n.get(),
            Err(err) => {
                error!("failed to get avaialble parallelism count: {}. fallback to default 1", err);
                1_usize
            }
        };

        for _ in 0..cpu_cores {
            let self_c = self.clone();
            let itx_c = itx.clone();
            let orx = orx.clone();
            let tun_dev_tx = tun_dev_tx.clone();

            tokio::spawn(async move {
                while let Ok((peer, mut payload)) = orx.recv().await {
                    if payload.len() <= HEADER_SIZE {
                        debug!("{} packet omitted, unusual size: {}b", hex::encode(&payload), payload.len());
                        continue;
                    }

                    let header_buffer = payload.split_to(HEADER_SIZE);
                    let header_buffer: [u8; HEADER_SIZE] = header_buffer[..].try_into().unwrap();
                    let header_frame = header::decode(header_buffer);

                    let identity = match device::util::get_source_identity(&payload) {
                        Some(identity) => identity,
                        None => {
                            debug!("{} packet omitted, source identity not found", hex::encode(&payload));
                            continue;
                        }
                    };

                    debug!(
                        "{} bytes read from {}, identity: {}, header frame: {:?}",
                        payload.len() + HEADER_SIZE,
                        peer,
                        identity,
                        header_frame
                    );

                    if !header_frame.primary_node_ip.is_unspecified() && (header_frame.primary_node_ip != self_c.machine_addr) {
                        let primary_node_addr = SocketAddr::new(IpAddr::V4(header_frame.primary_node_ip), header_frame.primary_node_port);
                        let self_c = self_c.clone();

                        self_c.route_to(primary_node_addr, &payload, itx_c.clone()).await.unwrap();
                    } else {
                        let _ = tun_dev_tx.send(payload).await;
                    }

                    // FIXME(nosiee): we need to use, let's call it a contolling table
                    // something that wireguard do. each client has an id and allowed ip from
                    // private network. the range is unique for each client
                    // after the packet from client arrived here and we validate ip/client_id
                    // we can create the peers mapping and to be sure that there is no other
                    // way to overwrite it only if an attacker have client_id and ip range
                    self_c.add_coordination(identity, peer).await;
                }
            });
        }

        tokio::spawn(async move {
            while let Ok(payload) = tun_dev_rx.recv().await {
                let identity = match device::util::get_destination_identity(&payload) {
                    Some(identity) => identity,
                    None => {
                        debug!("{} packet omitted, destination identity not found", hex::encode(&payload));
                        continue;
                    }
                };

                match self.get_coordination(&identity).await {
                    Some(peer) => {
                        itx.send((peer, payload)).await.unwrap();
                    }
                    None => debug!("{} packet omitted, coordination not found", hex::encode(&payload)),
                }
            }
        });

        (otx, irx)
    }

    async fn route_to(self: Arc<Self>, addr: SocketAddr, payload: &[u8], itx: Sender<PacketCoordinatorMessage>) -> anyhow::Result<(), TunnelError> {
        let node = match self.nodes.iter().find(|n| n.addr == addr) {
            Some(node) => node.clone(),
            None => return Err(TunnelError::Connection(("no such node".into(), NO_PEER_FOUND))),
        };

        debug!(
            "{} route the packet to the {} primary node",
            self.machine_addr.to_string(),
            addr.to_string()
        );

        let primary_nodes_guard = self.primary_nodes.read().await;
        let self_c = self.clone();

        if primary_nodes_guard.get(&addr.to_string()).is_some() {
            let _ = node.tunnel.send(payload).await.unwrap();
        } else {
            drop(primary_nodes_guard);
            let _ = node.tunnel.send(payload).await.unwrap();

            tokio::spawn(async move {
                loop {
                    let mut buffer = BytesMut::zeroed(node.max_fragment_size);

                    if let Ok(n) = node.tunnel.recv(&mut buffer).await {
                        buffer.truncate(n);
                        debug!("{} bytes read from {}", n, node.addr.to_string());

                        let identity = match device::util::get_destination_identity(&buffer) {
                            Some(identity) => identity,
                            None => {
                                debug!("{} packet omitted, destination identity not found", hex::encode(&buffer));
                                continue;
                            }
                        };

                        match self_c.get_coordination(&identity).await {
                            Some(peer) => {
                                itx.send((peer, buffer.freeze())).await.unwrap();
                            }
                            None => debug!("{} packet omitted, coordination not found", hex::encode(&buffer)),
                        }
                    }
                }
            });

            let mut primary_nodes_guard = self.primary_nodes.write().await;
            primary_nodes_guard.insert(addr.to_string(), ());
        }

        Ok(())
    }

    async fn add_coordination(&self, identity: String, peer: String) {
        let mut table_guard = self.coordination_table.write().await;
        table_guard.insert(identity, peer);
    }

    async fn get_coordination(&self, identity: &String) -> Option<String> {
        let table_guard = self.coordination_table.read().await;
        table_guard.get(identity).cloned()
    }
}
