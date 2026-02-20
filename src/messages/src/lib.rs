pub mod player_damage;

use bevy_ecs::message::MessageRegistry;
use bevy_ecs::prelude::World;
pub use player_damage::*;

pub mod player_digging;
pub use player_digging::*;

pub mod player_eat;
pub use player_eat::*;

pub mod player_exp;
pub use player_exp::*;

pub mod player_join;
pub use player_join::*;

pub mod player_leave;
pub use player_leave::*;

pub mod change_gamemode;
pub mod chunk_calc;

pub use change_gamemode::*;

pub mod entity_spawn;
pub mod entity_update;
pub mod particle;

pub use entity_spawn::{EntityType, SpawnEntityCommand, SpawnEntityEvent};

pub mod block_break;
pub mod cross_chunk_boundary_event;
pub mod force_player_recount_event;
pub mod packet_messages;
pub mod teleport_player;
pub mod world_change;

use crate::chunk_calc::ChunkCalc;
use crate::entity_update::SendEntityUpdate;
use crate::force_player_recount_event::ForcePlayerRecount;
use crate::packet_messages::Movement;
use crate::particle::SendParticle;
use crate::teleport_player::TeleportPlayer;
pub use block_break::BlockBrokenEvent;
use temper_commands::messages::{CommandDispatched, ResolvedCommandDispatched};
use world_change::WorldChange;

pub fn register_messages(world: &mut World) {
    MessageRegistry::register_message::<Movement>(world);
    MessageRegistry::register_message::<ChunkCalc>(world);
    MessageRegistry::register_message::<ForcePlayerRecount>(world);
    MessageRegistry::register_message::<CommandDispatched>(world);
    MessageRegistry::register_message::<ResolvedCommandDispatched>(world);

    MessageRegistry::register_message::<PlayerLeft>(world);
    MessageRegistry::register_message::<PlayerJoined>(world);
    MessageRegistry::register_message::<PlayerDamaged>(world);
    MessageRegistry::register_message::<PlayerDied>(world);
    MessageRegistry::register_message::<PlayerStartedDigging>(world);
    MessageRegistry::register_message::<PlayerCancelledDigging>(world);
    MessageRegistry::register_message::<PlayerFinishedDigging>(world);
    MessageRegistry::register_message::<PlayerEating>(world);
    MessageRegistry::register_message::<PlayerGainedXP>(world);
    MessageRegistry::register_message::<PlayerLeveledUp>(world);
    MessageRegistry::register_message::<PlayerGameModeChanged>(world);
    MessageRegistry::register_message::<SpawnEntityCommand>(world);
    MessageRegistry::register_message::<SpawnEntityEvent>(world);
    MessageRegistry::register_message::<SendEntityUpdate>(world);
    MessageRegistry::register_message::<SendParticle>(world);
    MessageRegistry::register_message::<BlockBrokenEvent>(world);
    MessageRegistry::register_message::<TeleportPlayer>(world);
    MessageRegistry::register_message::<WorldChange>(world);
}
