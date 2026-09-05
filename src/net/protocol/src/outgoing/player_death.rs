use temper_codec::net_types::var_int::VarInt;
use temper_macros::{packet, NetEncode};
use temper_nbt::NBT;
use temper_text::TextComponent;

#[derive(NetEncode)]
#[packet(packet_id = "player_combat_kill", state="play")]
pub struct PlayerDeath {
    pub entity_id: VarInt,
    /// This is the message displayed on the respawn screen, death messages are handled elsewhere 
    pub message: NBT<TextComponent>,
}
