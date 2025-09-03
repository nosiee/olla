pub mod checksum;
pub mod config;

use config::DeviceConfig;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tun_rs::{AsyncDevice, DeviceBuilder, Layer};

pub const DEVICE_BUFFER_SIZE: usize = 1024;
pub type Message = Vec<u8>;

pub struct Device {
    dev: Arc<AsyncDevice>,
    disable_on_exit: bool,
    total_buffer_size: u16,
}

impl Device {
    pub fn new_tun(config: DeviceConfig) -> anyhow::Result<Self> {
        let disable_on_exit = config.disable_on_exit;
        let total_buffer_size = config.mtu;

        let dev = Self::build_dev(config, Layer::L3)?;

        Ok(Self {
            dev: Arc::new(dev),
            disable_on_exit,
            total_buffer_size,
        })
    }

    pub fn new_tap(config: DeviceConfig) -> anyhow::Result<Self> {
        let disable_on_exit = config.disable_on_exit;
        let total_buffer_size = config.mtu;

        let dev = Self::build_dev(config, Layer::L2)?;

        Ok(Self {
            dev: Arc::new(dev),
            disable_on_exit,
            total_buffer_size,
        })
    }

    pub async fn forward(&self) -> anyhow::Result<(Sender<Message>, Receiver<Message>)> {
        let (itx, irx): (Sender<Message>, Receiver<Message>) = mpsc::channel(DEVICE_BUFFER_SIZE);
        let (otx, mut orx): (Sender<Message>, Receiver<Message>) = mpsc::channel(DEVICE_BUFFER_SIZE);
        let total_buffer_size = self.total_buffer_size.into();

        let in_dev = self.dev.clone();
        let out_dev = self.dev.clone();

        tokio::spawn(async move {
            let mut buffer = vec![0; total_buffer_size];

            loop {
                if let Ok(n) = in_dev.recv(&mut buffer).await {
                    if let Err(err) = itx.send(buffer[0..n].to_vec()).await {
                        panic!("{:?}", err);
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Some(payload) = orx.recv().await {
                if let Err(err) = out_dev.send(payload.as_slice()).await {
                    panic!("{:?}", err);
                }
            }
        });

        Ok((otx, irx))
    }

    fn build_dev(config: DeviceConfig, layer: Layer) -> anyhow::Result<AsyncDevice> {
        Ok(DeviceBuilder::new()
            .name(config.name)
            .mtu(config.mtu)
            .ipv4(config.addr, config.mask, None)
            .layer(layer)
            .build_async()?)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        if self.disable_on_exit {
            self.dev.enabled(false).unwrap()
        }
    }
}
