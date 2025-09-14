use serde_derive::Deserialize;
use std::{fs, path::PathBuf};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub device: DeviceConfig,
    pub tunnel: Option<TunnelConfig>,
    pub rules: Option<ClientRules>,
    pub nodes: Vec<NodeConfig>,
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
    pub addr: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClientRules {
    pub tunnels: Vec<String>,
    pub nodes: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NodeConfig {
    pub id: String,
    pub addr: String,
    pub keepalive: Option<u64>,
    pub primary: Option<bool>,
}

pub fn from_file(path: PathBuf) -> anyhow::Result<Config> {
    let data = fs::read_to_string(path)?;
    Ok(toml::from_str(&data)?)
}
