use std::{collections::HashMap, sync::Arc};

use super::coordinator::{node::Node, rule::CoodinatorRules};
use crate::tunnels::tunnel::AsyncTunnel;

use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::RwLock;

pub mod node;
pub mod rule;

pub type Message = Vec<u8>;

#[derive(Debug)]
pub struct NodeCoordinator<T: AsyncTunnel + Send + Sync + 'static> {
    nodes: Vec<Arc<Node<T>>>,
    subscribes: RwLock<HashMap<String, ()>>,
    rules: Option<CoodinatorRules>,
}

impl<T: AsyncTunnel + Send + Sync + 'static> NodeCoordinator<T> {
    pub fn new(nodes: Vec<Arc<Node<T>>>) -> Self {
        NodeCoordinator {
            nodes,
            rules: None,
            subscribes: RwLock::new(HashMap::new()),
        }
    }

    pub fn forward(self: Arc<Self>) -> (Sender<Message>, Receiver<Message>) {
        let (itx, irx): (Sender<Message>, Receiver<Message>) = mpsc::channel(1024);
        let (otx, mut orx): (Sender<Message>, Receiver<Message>) = mpsc::channel(1024);
        let self_c = self.clone();

        tokio::spawn(async move {
            while let Some(payload) = orx.recv().await {
                let node = self_c.pick_node();

                if let Err(_) = node.tunnel.send(payload).await {
                    todo!();
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
                    let mut buffer = [0; 16 * 1024];

                    let n = match node.tunnel.recv(&mut buffer).await {
                        Ok(n) => n,
                        Err(_) => continue,
                    };

                    if let Err(_) = itx.send(buffer[..n].to_vec()).await {
                        todo!();
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
