use temper_codec::net_types::angle::NetAngle;
use temper_codec::net_types::var_int::VarInt;
use temper_components::player::player_identity::PlayerIdentity;
use temper_components::player::rotation::Rotation;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode, Clone)]
#[packet(packet_id = "move_entity_rot", state = "play")]
pub struct UpdateEntityRotationPacket {
    pub entity_id: VarInt,
    pub yaw: NetAngle,
    pub pitch: NetAngle,
    pub on_ground: bool,
}
impl UpdateEntityRotationPacket {
    pub fn new(entity_id: &PlayerIdentity, new_rot: &Rotation, on_ground: bool) -> Self {
        Self {
            entity_id: VarInt::new(entity_id.short_uuid),
            yaw: NetAngle::from_degrees(new_rot.yaw as f64),
            pitch: NetAngle::from_degrees(new_rot.pitch as f64),
            on_ground,
        }
    }
}
