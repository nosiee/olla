use bytes::Bytes;

pub type Identity = String;
pub type DeviceMessage = Bytes;
pub type PacketCoordinatorMessage = (String, String, DeviceMessage);
