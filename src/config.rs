use serde_derive::Deserialize;
use std::fs;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub iface: IfaceConfig,
    pub client: ClientConfig,
    pub nodes: Vec<NodeConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IfaceConfig {
    pub name: String,
    pub mtu: u16,
    pub address: String,
    pub mask: String,
    pub disable_on_exit: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ClientConfig {
    pub tunnels: Vec<String>,
    pub nodes: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NodeConfig {
    pub id: String,
    pub addr: String,
    pub ca: Option<String>,
    pub sni: Option<String>,
    pub tunnel: String,
}

pub fn from_file(path: &str) -> anyhow::Result<Config> {
    let data = fs::read_to_string(path)?;
    Ok(toml::from_str(&data)?)
}
