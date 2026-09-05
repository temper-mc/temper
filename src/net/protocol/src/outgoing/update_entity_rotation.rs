use temper_codec::net_types::angle::NetAngle;
use temper_codec::net_types::var_int::VarInt;
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;
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
    pub fn new(entity_id: &GameID, new_rot: &Rotation, on_ground: bool) -> Self {
        Self {
            entity_id: entity_id.get(),
            yaw: NetAngle::from_degrees(f64::from(new_rot.yaw)),
            pitch: NetAngle::from_degrees(f64::from(new_rot.pitch)),
            on_ground,
        }
    }
}
