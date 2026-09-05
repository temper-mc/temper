use temper_codec::net_types::prefixed_optional::PrefixedOptional;
use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "damage_event", state = "play")]
pub struct DamagePlayer {
    entity_id: VarInt,
    source_type_id: VarInt,
    source_cause_id: VarInt,
    source_direct_id: VarInt,
    source_position: PrefixedOptional<(f64, f64, f64)>,
}
