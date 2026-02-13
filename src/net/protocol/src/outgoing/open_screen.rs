use temper_nbt::NBT;
use temper_text::TextComponent;
use temper_codec::net_types::var_int::VarInt;
use temper_macros::{packet, NetEncode};

#[derive(NetEncode)]
#[packet(packet_id = "open_screen", state = "play")]
pub struct OpenScreen {
    pub window_id: VarInt,
    pub window_type: VarInt,
    pub title: NBT<TextComponent>,
}
