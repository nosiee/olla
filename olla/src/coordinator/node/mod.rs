pub mod rule;

use async_channel::{Receiver, Sender};
use bytes::BytesMut;
use std::net::SocketAddr;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, error};

use super::node::rule::CoodinatorRules;
use crate::device::{DEVICE_BUFFER_SIZE, Message};
use crate::tunnels::outgoing::OutgoingTunnel;

#[derive(Debug)]
pub struct Node {
    pub id: String,
    pub addr: SocketAddr,
    pub tunnel: OutgoingTunnel,
    pub max_fragment_size: usize,
}

#[derive(Debug)]
pub struct NodeCoordinator {
    nodes: Vec<Arc<Node>>,
    subscribes: RwLock<HashMap<String, ()>>,
    rules: Option<CoodinatorRules>,
}

impl NodeCoordinator {
    pub fn new(nodes: Vec<Arc<Node>>) -> Self {
        NodeCoordinator {
            nodes,
            rules: None,
            subscribes: RwLock::new(HashMap::new()),
        }
    }

    pub fn forward(self: Arc<Self>) -> (Sender<Message>, Receiver<Message>) {
        let (itx, irx): (Sender<Message>, Receiver<Message>) = async_channel::bounded(DEVICE_BUFFER_SIZE);
        let (otx, orx): (Sender<Message>, Receiver<Message>) = async_channel::bounded(DEVICE_BUFFER_SIZE);
        let self_c = self.clone();

        tokio::spawn(async move {
            while let Ok(payload) = orx.recv().await {
                let node = self_c.pick_node();
                debug!("{}, {} node picked", node.id, node.addr.to_string());

                if let Err(err) = node.tunnel.send(&payload).await {
                    error!("failed to send payload to {}: {:?}", node.addr.to_string(), err);
                    continue;
                }

                debug!("{} bytes written to {}", payload.len(), node.addr.to_string());
                self_c.subscribe_to_node(node, itx.clone()).await;
            }
        });

        (otx, irx)
    }

    async fn subscribe_to_node(&self, node: Arc<Node>, itx: Sender<Message>) {
        let r_guard = self.subscribes.read().await;
        let node_id = node.id.clone();

        if r_guard.get(&node_id).is_none() {
            drop(r_guard);

            let mut w_guard = self.subscribes.write().await;
            w_guard.insert(node_id, ());
            debug!("subscribed to {}, {} node", node.id, node.addr.to_string());

            tokio::spawn(async move {
                loop {
                    let mut buffer = BytesMut::zeroed(node.max_fragment_size);

                    if let Ok(n) = node.tunnel.recv(&mut buffer).await {
                        buffer.truncate(n);
                        debug!("{} bytes read from {}", n, node.addr.to_string());

                        if let Err(err) = itx.send(buffer.freeze()).await {
                            panic!("{}", err);
                        }
                    }
                }
            });
        }
    }

    fn pick_node(&self) -> Arc<Node> {
        if self.rules.is_some() {
            return self.pick_policy_node();
        }

        self.pick_random_node()
    }

    fn pick_random_node(&self) -> Arc<Node> {
        let n = self.nodes.len();
        let rand_n = rand::random_range(0..n);

        self.nodes[rand_n].clone()
    }

    fn pick_policy_node(&self) -> Arc<Node> {
        self.nodes[0].clone()
    }
}
