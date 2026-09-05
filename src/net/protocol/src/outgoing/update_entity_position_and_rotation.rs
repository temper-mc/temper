use temper_codec::net_types::angle::NetAngle;
use temper_codec::net_types::var_int::VarInt;
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;
use temper_components::player::rotation::Rotation;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode, Clone)]
#[packet(packet_id = "move_entity_pos_rot", state = "play")]
pub struct UpdateEntityPositionAndRotationPacket {
    pub entity_id: VarInt,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub yaw: NetAngle,
    pub pitch: NetAngle,
    pub on_ground: bool,
}

impl UpdateEntityPositionAndRotationPacket {
    pub fn new(
        entity_id: &GameID,
        delta_positions: (i16, i16, i16),
        new_rot: &Rotation,
        on_ground: bool,
    ) -> Self {
        Self {
            entity_id: entity_id.get(),
            delta_x: delta_positions.0,
            delta_y: delta_positions.1,
            delta_z: delta_positions.2,
            yaw: NetAngle::from_degrees(f64::from(new_rot.yaw)),
            pitch: NetAngle::from_degrees(f64::from(new_rot.pitch)),
            on_ground,
        }
    }
}
