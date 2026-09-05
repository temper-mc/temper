use bevy_ecs::prelude::{Entity, Message};
use std::time::Duration;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_core::block_state_id::BlockStateId;
use temper_core::pos::BlockPos;
use temper_data::damage_types::DamageType;
use temper_macros::match_block;

#[derive(Message)]
pub struct DamageEvent {
    target: Entity,
    source: DamageSource,
    damage: u16,
    knockback: Option<f32>,
    knockback_source: Option<Position>,
}

pub enum DamageSource {
    Arrow {
        shooter: Option<Entity>,
    },
    Cramming {
        other_mobs: Vec<Entity>,
    },
    DragonBreath {
        dragon: Entity,
    },
    Drown,
    DryOut,
    EnderPearl {
        thrown_from: (Position, Rotation),
    },
    Explosion {
        source: Option<Entity>,
    },
    Fall {
        last_ground: Option<Position>,
    },
    FallingAnvil {
        anvil: Entity,
    },
    FallingBlock {
        entity: Entity,
        block: BlockStateId,
    },
    FallingStalactite {
        entity: Entity,
    },
    Fireball {
        shooter: Option<Entity>,
    },
    FlyIntoWall {
        block_hit: BlockPos,
    },
    Freeze,
    Generic,
    BlockDamage {
        block_pos: BlockPos,
        block_type: BlockStateId,
    },
    Suffocation,
    Lava,
    Lightning {
        entity: Entity,
    },
    MaceSmash {
        wielder: Entity,
    },
    Magic {
        caster: Entity,
    },

    // TODO: Weapon used
    MobAttack {
        attacker: Entity,
    },
    PlayerAttack {
        player: Entity,
    },

    Burned {
        burn_time_left: Duration,
    },
    SonicBoom {
        warden: Option<Entity>,
    },
    SpatOn {
        llama: Option<Entity>,
    },
    Starve,
    BeeSting {
        bee: Entity,
    },

    /// Returned damage is how much damage you dealt before some got returned
    Thorns {
        thorny_entity: Entity,
        returned_damage: u16,
    },
    ThrownTrident {
        thrower: Option<Entity>,
        trident_entity: Entity,
    },
    WindCharge {
        thrower: Option<Entity>,
        wind_charge_entity: Entity,
    },
    WitheredAway {
        inflicter: Option<Entity>,
        from_wither_skull: bool,
    },
    WitherSkullExplosion {
        shooting_wither: Option<Entity>,
        wither_skull_entity: Entity,
    },

    // Custom sources
    DivineSmiting {
        silent: bool,
    },
    CombatFallDamage {
        attacker: Entity,
        hit_from: Position,
    },
}

impl DamageSource {
    pub fn to_vanilla_source(&self) -> DamageType {
        match self {
            Self::Arrow { .. } => DamageType::Arrow,
            Self::Cramming { .. } => DamageType::Cramming,
            Self::DragonBreath { .. } => DamageType::DragonBreath,
            Self::Drown => DamageType::Drown,
            Self::DryOut => DamageType::DryOut,
            Self::EnderPearl { .. } => DamageType::EnderPearl,
            Self::Explosion { .. } => DamageType::Explosion,
            Self::Fall { .. } => DamageType::Fall,
            Self::FallingAnvil { .. } => DamageType::FallingAnvil,
            Self::FallingBlock { .. } => DamageType::FallingBlock,
            Self::FallingStalactite { .. } => DamageType::FallingStalactite,
            Self::Fireball { shooter: Some(_) } => DamageType::Fireball,
            Self::Fireball { shooter: None } => DamageType::UnattributedFireball,
            Self::FlyIntoWall { .. } => DamageType::FlyIntoWall,
            Self::Freeze => DamageType::Freeze,
            Self::Generic => DamageType::Generic,
            Self::BlockDamage { block_type, .. } => Self::block_damage_source(*block_type),
            Self::Suffocation => DamageType::InWall,
            Self::Lava => DamageType::Lava,
            Self::Lightning { .. } => DamageType::LightningBolt,
            Self::MaceSmash { .. } => DamageType::MaceSmash,
            Self::Magic { .. } => DamageType::Magic,
            Self::MobAttack { .. } => DamageType::MobAttack,
            Self::PlayerAttack { .. } => DamageType::PlayerAttack,
            Self::Burned { .. } => DamageType::OnFire,
            Self::SonicBoom { .. } => DamageType::SonicBoom,
            Self::SpatOn { .. } => DamageType::Spit,
            Self::Starve => DamageType::Starve,
            Self::BeeSting { .. } => DamageType::Sting,
            Self::Thorns { .. } => DamageType::Thorns,
            Self::ThrownTrident { .. } => DamageType::Trident,
            Self::WindCharge { .. } => DamageType::WindCharge,
            Self::WitheredAway { .. } => DamageType::Wither,
            Self::WitherSkullExplosion { .. } => DamageType::WitherSkull,
            _ => DamageType::Generic,
        }
    }

    fn block_damage_source(block_type: BlockStateId) -> DamageType {
        if match_block!("cactus", block_type) {
            DamageType::Cactus
        } else if match_block!("campfire", block_type) || match_block!("soul_campfire", block_type)
        {
            DamageType::Campfire
        } else if match_block!("magma_block", block_type) {
            DamageType::HotFloor
        } else if match_block!("sweet_berry_bush", block_type) {
            DamageType::SweetBerryBush
        } else if match_block!("fire", block_type) || match_block!("soul_fire", block_type) {
            DamageType::InFire
        } else if match_block!("lava", block_type) {
            DamageType::Lava
        } else {
            DamageType::Generic
        }
    }
}
