use temper_codec::net_types::var_int::VarInt;
use temper_components::game_id::GameID;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode, Clone)]
#[packet(packet_id = "move_entity_pos", state = "play")]
pub struct UpdateEntityPositionPacket {
    pub entity_id: VarInt,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub on_ground: bool,
}

impl UpdateEntityPositionPacket {
    pub fn new(entity_id: &GameID, delta_positions: (i16, i16, i16), on_ground: bool) -> Self {
        Self {
            entity_id: entity_id.get(),
            delta_x: delta_positions.0,
            delta_y: delta_positions.1,
            delta_z: delta_positions.2,
            on_ground,
        }
    }
}
