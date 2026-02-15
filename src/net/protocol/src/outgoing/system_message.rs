use temper_macros::{NetEncode, packet};
use temper_text::TextComponent;

#[derive(NetEncode, Debug, Clone)]
#[packet(packet_id = "system_chat", state = "play")]
pub struct SystemMessagePacket {
    pub message: temper_nbt::NBT<TextComponent>,
    pub overlay: bool,
}
