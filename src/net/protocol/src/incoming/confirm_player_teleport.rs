use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetDecode, packet};

#[derive(NetDecode)]
#[packet(packet_id = "accept_teleportation", state = "play")]
pub struct ConfirmPlayerTeleport {
    pub teleport_id: VarInt,
}
