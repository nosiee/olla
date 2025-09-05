#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub name: String,
    pub mtu: u16,
    pub addr: String,
    pub mask: String,
    pub disable_on_exit: bool,
}
