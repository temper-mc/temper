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
    pub target: Entity,
    pub source: DamageSource,
    pub damage: f32,
    pub knockback: Option<f32>,
    pub knockback_source: Option<Position>,
}
#[derive(Clone)]
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

    pub fn to_death_message(&self, victim_name: &str, attacker_name: Option<&str>) -> String {
        match self {
            Self::Arrow { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was shot by {attacker}"),
                None => format!("{victim_name} was shot by an arrow"),
            },
            Self::Cramming { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was squished by {attacker}"),
                None => format!("{victim_name} was squished too much"),
            },
            Self::DragonBreath { .. } => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} was roasted in dragon breath by {attacker}")
                }
                None => format!("{victim_name} was roasted in dragon breath"),
            },
            Self::Drown => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} drowned whilst trying to escape {attacker}")
                }
                None => format!("{victim_name} drowned"),
            },
            Self::DryOut => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} dried out whilst trying to escape {attacker}")
                }
                None => format!("{victim_name} dried out"),
            },
            Self::EnderPearl { .. } => match attacker_name {
                Some(attacker) => format!(
                    "{victim_name} hit the ground too hard whilst trying to escape {attacker}"
                ),
                None => format!("{victim_name} hit the ground too hard whilst trying to escape"),
            },
            Self::Explosion { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was blown up by {attacker}"),
                None => format!("{victim_name} blew up"),
            },
            Self::Fall { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was pushed off a cliff by {attacker}"),
                None => format!("{victim_name} fell from a high place"),
            },
            Self::FallingAnvil { .. } => match attacker_name {
                Some(attacker) => format!(
                    "{victim_name} was squashed by a falling anvil while fighting {attacker}"
                ),
                None => format!("{victim_name} was squashed by a falling anvil"),
            },
            Self::FallingBlock { block, .. } => {
                match attacker_name {
                    Some(attacker) => {
                        format!("{victim_name} was squashed by a falling block while fighting {attacker}")
                    }
                    None => format!("{victim_name} was squashed by a falling block ({block:?})"),
                }
            }
            Self::FallingStalactite { .. } => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} was impaled by a falling stalactite while fighting {attacker}")
                }
                None => format!("{victim_name} was impaled by a falling stalactite"),
            },
            Self::Fireball { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was fireballed by {attacker}"),
                None => format!("{victim_name} was fireballed"),
            },
            Self::FlyIntoWall { .. } => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} experienced kinetic energy while trying to escape {attacker}")
                }
                None => format!("{victim_name} experienced kinetic energy"),
            },
            Self::Freeze => match attacker_name {
                Some(attacker) => format!("{victim_name} froze to death by {attacker}"),
                None => format!("{victim_name} froze to death"),
            },
            Self::Generic => match attacker_name {
                Some(attacker) => format!("{victim_name} died because of {attacker}"),
                None => format!("{victim_name} died"),
            },
            Self::BlockDamage { block_type, .. } => {
                Self::block_death_message(victim_name, *block_type, attacker_name)
            }
            Self::Suffocation => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} suffocated in a wall while fighting {attacker}")
                }
                None => format!("{victim_name} suffocated in a wall"),
            },
            Self::Lava => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} tried to swim in lava to escape {attacker}")
                }
                None => format!("{victim_name} tried to swim in lava"),
            },
            Self::Lightning { .. } => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} was struck by lightning while fighting {attacker}")
                }
                None => format!("{victim_name} was struck by lightning"),
            },
            Self::MaceSmash { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was smashed from above by {attacker}"),
                None => format!("{victim_name} was smashed from above"),
            },
            Self::Magic { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was killed by magic used by {attacker}"),
                None => format!("{victim_name} was killed by magic"),
            },
            Self::MobAttack { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was slain by {attacker}"),
                None => format!("{victim_name} was slain"),
            },
            Self::PlayerAttack { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was slain by {attacker}"),
                None => format!("{victim_name} was slain in combat"),
            },
            Self::Burned { .. } => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} was burned to a crisp whilst fighting {attacker}")
                }
                None => format!("{victim_name} burned to death"),
            },
            Self::SonicBoom { .. } => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} was obliterated by a sonically-charged shriek from {attacker}")
                }
                None => format!("{victim_name} was obliterated by a sonically-charged shriek"),
            },
            Self::SpatOn { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was spat on by {attacker}"),
                None => format!("{victim_name} was spat on"),
            },
            Self::Starve => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} starved to death while fighting {attacker}")
                }
                None => format!("{victim_name} starved to death"),
            },
            Self::BeeSting { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was stung to death by {attacker}"),
                None => format!("{victim_name} was stung to death"),
            },
            Self::Thorns { .. } => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} was killed trying to hurt {attacker}")
                }
                None => format!("{victim_name} was killed trying to hurt their enemy"),
            },
            Self::ThrownTrident { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was impaled by {attacker}"),
                None => format!("{victim_name} was impaled"),
            },
            Self::WindCharge { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was blown away by {attacker}"),
                None => format!("{victim_name} was blown away"),
            },
            Self::WitheredAway { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} withered away while fighting {attacker}"),
                None => format!("{victim_name} withered away"),
            },
            Self::WitherSkullExplosion { .. } => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} was shot by a wither skull from {attacker}")
                }
                None => format!("{victim_name} was shot by a wither skull"),
            },
            Self::DivineSmiting { silent: _ } => match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} was struck down by divine intervention from {attacker}")
                }
                None => format!("{victim_name} was struck down by divine intervention"),
            },
            Self::CombatFallDamage { .. } => match attacker_name {
                Some(attacker) => format!("{victim_name} was doomed to fall by {attacker}"),
                None => format!("{victim_name} was doomed to fall in combat"),
            },
        }
    }

    fn block_death_message(
        victim_name: &str,
        block_type: BlockStateId,
        attacker_name: Option<&str>,
    ) -> String {
        if match_block!("cactus", block_type) {
            match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} walked into a cactus whilst trying to escape {attacker}")
                }
                None => format!("{victim_name} was pricked to death"),
            }
        } else if match_block!("campfire", block_type) || match_block!("soul_campfire", block_type)
        {
            match attacker_name {
                Some(attacker) => {
                    format!(
                        "{victim_name} walked into a campfire whilst trying to escape {attacker}"
                    )
                }
                None => format!("{victim_name} stepped on a campfire"),
            }
        } else if match_block!("magma_block", block_type) {
            match attacker_name {
                Some(attacker) => format!("{victim_name} walked on danger zone due to {attacker}"),
                None => format!("{victim_name} discovered floor was lava"),
            }
        } else if match_block!("sweet_berry_bush", block_type) {
            match attacker_name {
                Some(attacker) => format!(
                    "{victim_name} was poked to death by a sweet berry bush whilst trying to escape {attacker}"
                ),
                None => format!("{victim_name} was poked to death by a sweet berry bush"),
            }
        } else if match_block!("fire", block_type) || match_block!("soul_fire", block_type) {
            match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} walked into fire whilst fighting {attacker}")
                }
                None => format!("{victim_name} went up in flames"),
            }
        } else if match_block!("lava", block_type) {
            match attacker_name {
                Some(attacker) => {
                    format!("{victim_name} tried to swim in lava to escape {attacker}")
                }
                None => format!("{victim_name} tried to swim in lava"),
            }
        } else {
            match attacker_name {
                Some(attacker) => format!("{victim_name} died because of {attacker}"),
                None => format!("{victim_name} died"),
            }
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
