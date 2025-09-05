pub mod config;

use bytes::{Bytes, BytesMut};
use config::DeviceConfig;
use std::sync::Arc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::debug;
use tun_rs::{AsyncDevice, DeviceBuilder, Layer};

pub const DEVICE_BUFFER_SIZE: usize = 1024;
pub type Message = Bytes;

pub struct Device {
    dev: Arc<AsyncDevice>,
    disable_on_exit: bool,
    mtu: u16,
}

impl Device {
    pub fn new_tun(config: DeviceConfig) -> anyhow::Result<Self> {
        let disable_on_exit = config.disable_on_exit;
        let mtu = config.mtu;
        let dev = Self::build_dev(config, Layer::L3)?;

        debug!(
            "new device was created: [{:?}, {:?}, {:?}, {:?}]",
            dev.name(),
            dev.mtu(),
            dev.addresses(),
            Layer::L3
        );

        Ok(Self {
            dev: Arc::new(dev),
            disable_on_exit,
            mtu,
        })
    }

    pub fn new_tap(config: DeviceConfig) -> anyhow::Result<Self> {
        let disable_on_exit = config.disable_on_exit;
        let mtu = config.mtu;
        let dev = Self::build_dev(config, Layer::L2)?;

        debug!(
            "new device was created: [{:?}, {:?}, {:?}, {:?}]",
            dev.name(),
            dev.mtu(),
            dev.addresses(),
            Layer::L2
        );

        Ok(Self {
            dev: Arc::new(dev),
            disable_on_exit,
            mtu,
        })
    }

    pub async fn forward(&self) -> anyhow::Result<(Sender<Message>, Receiver<Message>)> {
        let (itx, irx): (Sender<Message>, Receiver<Message>) = mpsc::channel(DEVICE_BUFFER_SIZE);
        let (otx, mut orx): (Sender<Message>, Receiver<Message>) = mpsc::channel(DEVICE_BUFFER_SIZE);
        let mtu = self.mtu;

        let in_dev = self.dev.clone();
        let out_dev = self.dev.clone();

        let dev_name = self.dev.name().unwrap_or_default();
        tokio::spawn(async move {
            loop {
                let mut buffer = BytesMut::zeroed(mtu as usize);

                if let Ok(n) = in_dev.recv(&mut buffer).await {
                    debug!("{} bytes read from {}", n, dev_name);

                    buffer.truncate(n);

                    if let Err(err) = itx.send(buffer.freeze()).await {
                        panic!("{:?}", err);
                    }
                }
            }
        });

        let dev_name = self.dev.name().unwrap_or_default();
        tokio::spawn(async move {
            while let Some(payload) = orx.recv().await {
                if let Err(err) = out_dev.send(&payload).await {
                    panic!("{:?}", err);
                }

                debug!("{} bytes written to {}", payload.len(), dev_name);
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
