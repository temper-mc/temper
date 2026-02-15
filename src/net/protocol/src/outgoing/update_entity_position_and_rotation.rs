use temper_codec::net_types::angle::NetAngle;
use temper_codec::net_types::var_int::VarInt;
use temper_components::player::player_identity::PlayerIdentity;
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
        entity_id: &PlayerIdentity,
        delta_positions: (i16, i16, i16),
        new_rot: &Rotation,
        on_ground: bool,
    ) -> Self {
        Self {
            entity_id: VarInt::new(entity_id.short_uuid),
            delta_x: delta_positions.0,
            delta_y: delta_positions.1,
            delta_z: delta_positions.2,
            yaw: NetAngle::from_degrees(new_rot.yaw as f64),
            pitch: NetAngle::from_degrees(new_rot.pitch as f64),
            on_ground,
        }
    }
}
