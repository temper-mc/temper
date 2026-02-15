use temper_macros::{NetEncode, packet};
use temper_nbt::NBT;
use temper_text::{ComponentBuilder, TextComponent};

#[derive(NetEncode)]
#[packet(packet_id = "disconnect", state = "play")]
pub struct DisconnectPacket {
    pub reason: NBT<TextComponent>,
}

impl DisconnectPacket {
    pub fn new(reason: TextComponent) -> Self {
        Self {
            reason: NBT::new(reason),
        }
    }
    pub fn from_string(reason: String) -> Self {
        let reason = ComponentBuilder::text(reason);
        Self {
            reason: NBT::new(reason.build()),
        }
    }
}

impl Default for DisconnectPacket {
    fn default() -> Self {
        Self::from_string("TEMPER-DISCONNECTED".to_string())
    }
}
