use crate::errors::NetError;
use bevy_ecs::prelude::{Entity, Query};
use temper_codec::net_types::angle::NetAngle;
use temper_codec::net_types::lpvec3::Lpvec3;
use temper_codec::net_types::var_int::VarInt;
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_components::player::velocity::Velocity;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "add_entity", state = "play")]
pub struct SpawnEntityPacket {
    entity_id: VarInt,
    entity_uuid: u128,
    r#type: VarInt,
    x: f64,
    y: f64,
    z: f64,
    velocity: Lpvec3,
    pitch: NetAngle,
    yaw: NetAngle,
    head_yaw: NetAngle,
    data: VarInt,
}

impl SpawnEntityPacket {
    /// Creates a spawn entity packet from direct component values.
    ///
    /// This is useful when you have the component values directly
    /// rather than needing to query them.
    pub fn new(
        entity_id: VarInt,
        entity_uuid: u128,
        entity_type_id: i32,
        position: &Position,
        rotation: &Rotation,
        velocity: &Velocity,
    ) -> Self {
        let (x, y, z) = position.xyz();
        let (yaw, pitch) = rotation.yaw_pitch();

        Self {
            entity_id,
            entity_uuid,
            r#type: VarInt::new(entity_type_id),
            x,
            y,
            z,
            velocity: velocity.vec.as_dvec3().into(),
            pitch: NetAngle::from_degrees(f64::from(pitch)),
            yaw: NetAngle::from_degrees(f64::from(yaw)),
            head_yaw: NetAngle::from_degrees(f64::from(yaw)),
            data: VarInt::new(0),
        }
    }

    /// Creates a spawn entity packet for any entity (mob, projectile, etc.).
    ///
    /// # Arguments
    ///
    /// * `entity` - Bevy entity to spawn
    /// * `entity_type_id` - Protocol ID for the entity type (from vanilla data)
    /// * `query` - Query to get entity components
    pub fn entity(
        entity: Entity,
        entity_type_id: u16,
        query: Query<(&Identity, &Position, &Rotation, &Velocity, &GameID)>,
    ) -> Result<Self, NetError> {
        let (identity, position, rotation, vel, game_id) = query
            .get(entity)
            .map_err(|e| NetError::ECSError(e.into()))?;

        let (x, y, z) = position.xyz();
        let (yaw, pitch) = rotation.yaw_pitch();

        Ok(Self {
            entity_id: game_id.get(),
            entity_uuid: identity.uuid.as_u128(),
            r#type: VarInt::new(i32::from(entity_type_id)),
            x,
            y,
            z,
            velocity: vel.vec.as_dvec3().into(),
            pitch: NetAngle::from_degrees(f64::from(pitch)),
            yaw: NetAngle::from_degrees(f64::from(yaw)),
            head_yaw: NetAngle::from_degrees(f64::from(yaw)),
            data: VarInt::new(0),
        })
    }
}
