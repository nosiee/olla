use super::coordinator::{node::Node, rule::CoodinatorRules};

use device::{DEVICE_BUFFER_SIZE, Message};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tunnels::{AsyncOutgoingTunnel, header};

pub mod node;
pub mod rule;

#[derive(Debug)]
pub struct NodeCoordinator<T: AsyncOutgoingTunnel + Send + Sync + 'static> {
    nodes: Vec<Arc<Node<T>>>,
    subscribes: RwLock<HashMap<String, ()>>,
    rules: Option<CoodinatorRules>,
}

impl<T: AsyncOutgoingTunnel + Send + Sync + 'static> NodeCoordinator<T> {
    pub fn new(nodes: Vec<Arc<Node<T>>>) -> Self {
        NodeCoordinator {
            nodes,
            rules: None,
            subscribes: RwLock::new(HashMap::new()),
        }
    }

    pub fn forward(self: Arc<Self>) -> (Sender<Message>, Receiver<Message>) {
        let (itx, irx): (Sender<Message>, Receiver<Message>) = mpsc::channel(DEVICE_BUFFER_SIZE);
        let (otx, mut orx): (Sender<Message>, Receiver<Message>) = mpsc::channel(DEVICE_BUFFER_SIZE);
        let self_c = self.clone();

        tokio::spawn(async move {
            while let Some(payload) = orx.recv().await {
                let node = self_c.pick_node();

                if let Err(err) = node.tunnel.send(payload).await {
                    panic!("{:?}", err);
                }

                self_c.subscribe_to_node(node, itx.clone()).await;
            }
        });

        (otx, irx)
    }

    async fn subscribe_to_node(&self, node: Arc<Node<T>>, itx: Sender<Message>) {
        let r_guard = self.subscribes.read().await;
        let node_id = node.id.clone();

        if r_guard.get(&node_id).is_none() {
            // NOTE(nosiee): not sure about this but otherwise we will block on
            // subscribes.write() call
            drop(r_guard);

            let mut w_guard = self.subscribes.write().await;
            w_guard.insert(node_id, ());

            tokio::spawn(async move {
                loop {
                    let mut header_buf = [0; header::HEADER_SIZE];

                    if node.tunnel.recv_exact(&mut header_buf).await.is_ok() {
                        let header_frame = header::decode(header_buf);
                        let payload_size: usize = header_frame.frame_size as usize - header::HEADER_SIZE;
                        let mut buf = vec![0; payload_size];

                        if node.tunnel.recv_exact(&mut buf).await.is_ok() {
                            if let Err(err) = itx.send(buf).await {
                                panic!("{}", err);
                            }
                        }
                    }
                }
            });
        }
    }

    fn pick_node(&self) -> Arc<Node<T>> {
        if self.rules.is_some() {
            return self.pick_policy_node();
        }

        self.pick_random_node()
    }

    fn pick_random_node(&self) -> Arc<Node<T>> {
        let n = self.nodes.len();
        let rand_n = rand::random_range(0..n);

        self.nodes[rand_n].clone()
    }

    fn pick_policy_node(&self) -> Arc<Node<T>> {
        self.nodes[0].clone()
    }
}
