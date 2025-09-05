use serde_derive::Deserialize;
use std::fs;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub device: DeviceConfig,
    pub tunnels: Vec<TunnelConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceConfig {
    pub name: String,
    pub mtu: u16,
    pub addr: String,
    pub mask: String,
    pub disable_on_exit: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TunnelConfig {
    pub tunnel_type: String,
    pub addr: String,

    pub cert: Option<String>,
    pub key: Option<String>,
}

pub fn from_file(path: &str) -> anyhow::Result<Config> {
    let data = fs::read_to_string(path)?;
    Ok(toml::from_str(&data)?)
}
