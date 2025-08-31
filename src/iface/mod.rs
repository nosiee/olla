use crate::config::IfaceConfig;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tun_rs::{AsyncDevice, DeviceBuilder, Layer};

pub type Message = Vec<u8>;

pub struct Interface {
    dev: Arc<AsyncDevice>,

    disable_on_exit: bool,
    total_buffer_size: u16,
}

impl Interface {
    pub fn new_tun(config: IfaceConfig) -> anyhow::Result<Self> {
        let disable_on_exit = config.disable_on_exit;
        let total_buffer_size = config.mtu + 4;

        let dev = Self::build_dev(config, Layer::L3)?;

        Ok(Self {
            dev: Arc::new(dev),
            disable_on_exit,
            total_buffer_size,
        })
    }

    pub fn new_tap(config: IfaceConfig) -> anyhow::Result<Self> {
        let disable_on_exit = config.disable_on_exit;
        let total_buffer_size = config.mtu + 4;

        let dev = Self::build_dev(config, Layer::L2)?;

        Ok(Self {
            dev: Arc::new(dev),
            disable_on_exit,
            total_buffer_size,
        })
    }

    pub async fn forward(&self) -> anyhow::Result<(Sender<Message>, Receiver<Message>)> {
        let (itx, irx): (Sender<Message>, Receiver<Message>) = mpsc::channel(1024);
        let (otx, mut orx): (Sender<Message>, Receiver<Message>) = mpsc::channel(1024);
        let total_buffer_size = self.total_buffer_size.into();

        let in_dev = self.dev.clone();
        let out_dev = self.dev.clone();

        tokio::spawn(async move {
            let mut buffer = vec![0; total_buffer_size];

            loop {
                if in_dev.recv(&mut buffer).await.is_ok() {
                    if let Err(_) = itx.send(buffer.clone()).await {
                        todo!();
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Some(payload) = orx.recv().await {
                if let Err(_) = out_dev.send(payload.as_slice()).await {
                    todo!();
                }
            }
        });

        Ok((otx, irx))
    }

    fn build_dev(config: IfaceConfig, layer: Layer) -> anyhow::Result<AsyncDevice> {
        Ok(DeviceBuilder::new()
            .name(config.name)
            .mtu(config.mtu)
            .ipv4(config.address, config.mask, None)
            .layer(layer)
            .build_async()?)
    }
}

impl Drop for Interface {
    fn drop(&mut self) {
        if self.disable_on_exit {
            self.dev.enabled(false).unwrap()
        }
    }
}
