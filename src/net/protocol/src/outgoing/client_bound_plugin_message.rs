use temper_codec::encode::NetEncode;
use temper_macros::{NetEncode, packet};

const SERVER_VERSION: &str = concat!("TEMPER v", env!("TEMPER_VERSION"), env!("BUILD_TYPE"));

type BrandChannelData = String;

#[derive(NetEncode, Clone)]
#[packet(packet_id = "custom_payload", state = "configuration")]
pub struct ClientBoundPluginMessagePacket<T: NetEncode> {
    pub channel: String,
    pub data: T,
}

impl ClientBoundPluginMessagePacket<BrandChannelData> {
    pub fn brand() -> ClientBoundPluginMessagePacket<BrandChannelData> {
        Self {
            channel: "minecraft:brand".to_string(),
            data: SERVER_VERSION.to_string(),
        }
    }
}

impl<T: NetEncode> ClientBoundPluginMessagePacket<T> {
    pub fn with_data_channel(
        channel: impl Into<String>,
        data: T,
    ) -> ClientBoundPluginMessagePacket<T> {
        Self {
            channel: channel.into(),
            data,
        }
    }
}
