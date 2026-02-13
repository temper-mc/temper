use bevy_ecs::prelude::ApplyDeferred;
use bevy_ecs::schedule::IntoScheduleConfigs;

mod chunk_calculator;
mod digging_system;
mod emit_player_joined;
mod entity_spawn;
mod gamemode_change;
mod movement_broadcast;
mod new_connections;
mod player_despawn;
mod player_join_message;
mod player_leave_message;
mod player_spawn;
mod player_swimming;
mod player_tp;
mod send_inventory_updates;
pub mod update_player_ping;

pub fn register_player_systems(schedule: &mut bevy_ecs::schedule::Schedule) {
    schedule.add_systems(chunk_calculator::handle);
    schedule.add_systems(digging_system::handle_start_digging);
    schedule.add_systems(digging_system::handle_finish_digging);
    schedule.add_systems(digging_system::handle_start_digging);
    schedule.add_systems(digging_system::handle_cancel_digging);
    schedule.add_systems(entity_spawn::handle_spawn_entity);
    schedule.add_systems(entity_spawn::spawn_command_processor);
    schedule.add_systems(gamemode_change::handle);
    schedule.add_systems(movement_broadcast::handle_player_move);

    // Player connection handling - chained to ensure proper event timing:
    // 1. accept_new_connections: Spawns entity + adds PendingPlayerJoin marker (deferred)
    // 2. ApplyDeferred: Flushes commands, entity now exists and is queryable
    // 3. emit_player_joined: Fires PlayerJoined event (listeners can now query the entity)
    schedule.add_systems(
        (
            new_connections::accept_new_connections,
            ApplyDeferred,
            emit_player_joined::emit_player_joined,
        )
            .chain(),
    );
    schedule.add_systems(player_despawn::handle);
    schedule.add_systems(player_join_message::handle);
    schedule.add_systems(player_leave_message::handle);
    schedule.add_systems(player_spawn::handle);
    schedule.add_systems(player_swimming::detect_player_swimming);
    schedule.add_systems(player_tp::teleport_player);
    schedule.add_systems(send_inventory_updates::handle_inventory_updates);
}
