use bevy_ecs::prelude::ApplyDeferred;
use bevy_ecs::schedule::{
    IntoScheduleConfigs, MultiThreadedExecutor, Schedule, SingleThreadedExecutor, SystemSet,
};
use std::time::Duration;
use temper_scheduler::{MissedTickBehavior, Scheduler, TimedSchedule, drain_registered_schedules};

pub use background::lan_pinger::LanPinger;
use temper_state::GlobalState;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum TickPhase {
    ChunkSending,
    MobSpawning,
    VisibleTracking,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum ChunkGcPhase {
    MarkForSave,
    UnloadChunks,
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum ShutdownPhase {
    EmitSaveMessages,
    FlushWorld,
    ShutdownPackets,
}

// TODO: Clean this up with bevy's app thing
fn register_tick_systems(schedule: &mut Schedule) {
    schedule.configure_sets(
        (
            TickPhase::ChunkSending,
            mobs::MobLoadSystems,
            TickPhase::MobSpawning,
            TickPhase::VisibleTracking,
        )
            .chain(),
    );

    schedule.add_systems(packets::chunk_batch_ack::handle);
    schedule.add_systems(packets::confirm_player_teleport::handle);
    schedule.add_systems(packets::keep_alive::handle);
    schedule.add_systems(packets::place_block::handle);
    schedule.add_systems(interactions::handle_block_interact);
    schedule.add_systems(packets::player_action::handle);
    schedule.add_systems(packets::player_command::handle);
    schedule.add_systems(packets::player_input::handle);
    schedule.add_systems(packets::set_player_position::handle);
    schedule.add_systems(packets::set_player_position_and_rotation::handle);
    schedule.add_systems(packets::set_player_rotation::handle);
    schedule.add_systems(packets::swing_arm::handle);
    schedule.add_systems(packets::update_survival_mode_slot::handle);
    schedule.add_systems(packets::close_container::handle);
    schedule.add_systems(packets::player_loaded::handle);
    schedule.add_systems(packets::command::handle);
    schedule.add_systems(packets::command_graph::rebuild_and_send_command_graphs);
    schedule.add_systems(packets::command_suggestions::handle);
    schedule.add_systems(packets::chat_message::handle);
    schedule.add_systems(packets::set_creative_mode_slot::handle);
    schedule.add_systems(packets::set_held_item::handle);
    schedule.add_systems(packets::player_abilities::handle);
    schedule.add_systems(packets::change_game_mode::handle);
    schedule.add_systems(packets::pick_item_from_block::handle);

    schedule.add_systems(player::digging_system::handle_start_digging);
    schedule.add_systems(player::digging_system::handle_finish_digging);
    schedule.add_systems(player::digging_system::handle_start_digging);
    schedule.add_systems(player::digging_system::handle_cancel_digging);
    schedule
        .add_systems(player::entity_spawn::spawn_command_processor.in_set(TickPhase::MobSpawning));
    schedule.add_systems(
        mobs::spawn::handle_spawn_mob_bundle
            .after(player::entity_spawn::spawn_command_processor)
            .in_set(TickPhase::MobSpawning),
    );
    schedule.add_systems(player::gamemode_change::handle);
    schedule.add_systems(player::movement_broadcast::handle_player_move);

    schedule.add_systems(
        (
            player::new_connections::accept_new_connections,
            ApplyDeferred,
            player::chunk_calculator::handle,
            player::emit_player_joined::emit_player_joined,
            player::player_spawn::handle,
        )
            .chain(),
    );
    schedule.add_systems(player::player_despawn::handle);
    schedule.add_systems(player::player_join_message::handle);
    schedule.add_systems(player::player_leave_message::handle);
    schedule.add_systems(player::player_swimming::detect_player_swimming);
    schedule.add_systems(player::teleport::teleport_entities);
    schedule.add_systems(player::send_inventory_updates::handle_inventory_updates);
    schedule.add_systems(player::damage_entity::damage_entity);

    temper_command_infra::register_command_systems(schedule);

    schedule.add_systems(background::chunk_sending::handle.in_set(TickPhase::ChunkSending));
    mobs::register_load_systems(schedule);
    schedule.add_systems(
        (
            background::entity_tracking::refresh_visible_entities,
            background::entity_sending::send_untracked_entities,
            background::entity_sending::send_new_entities,
            background::send_entity_updates::handle,
        )
            .chain()
            .in_set(TickPhase::VisibleTracking),
    );
    schedule.add_systems(background::connection_killer::connection_killer);
    schedule.add_systems(background::day_cycle::tick_daylight_cycle);
    schedule.add_systems(background::mq::process);
    schedule.add_systems(background::server_command::handle);
    schedule.add_systems(
        (
            background::kill_entity::kill_entity_system,
            mobs::spawn::handle_despawn_mob,
        )
            .chain(),
    );
    schedule.add_systems(background::generate_spawn_positions::generate_spawn_positions);

    schedule.add_systems(
        (
            physics::unground::handle,
            physics::gravity::handle,
            physics::drag::handle,
            physics::friction::handle,
            physics::velocity::handle,
            physics::collisions::handle,
            physics::chunk_boundary::handle,
            background::cross_chunk_border::cross_chunk_border,
        )
            .chain(),
    );
    mobs::register_tick_systems(schedule);

    schedule.add_systems(world::particles::handle);

    schedule.add_systems(background::bossbar_update::handle);

    schedule.add_systems(bevy_ecs::message::message_update_system);
}

fn register_world_sync_schedule_systems(schedule: &mut Schedule) {
    schedule.add_systems(background::world_sync::sync_world);
}

fn register_chunk_gc_schedule_systems(schedule: &mut Schedule) {
    schedule.set_executor(MultiThreadedExecutor::new());
    schedule.configure_sets(
        (
            ChunkGcPhase::MarkForSave,
            mobs::MobSaveSystems,
            ChunkGcPhase::UnloadChunks,
        )
            .chain(),
    );
    schedule.add_systems(
        (
            background::entity_unloader::handle,
            mobs::spawn::queue_live_mob_chunk_saves,
        )
            .chain()
            .in_set(ChunkGcPhase::MarkForSave),
    );
    mobs::register_save_systems(schedule);
    schedule.add_systems(
        (
            background::chunk_unloader::handle,
            mobs::spawn::handle_despawn_mob,
        )
            .chain()
            .in_set(ChunkGcPhase::UnloadChunks),
    );
}

fn register_keepalive_schedule_systems(schedule: &mut Schedule) {
    schedule.add_systems(background::keep_alive_system::keep_alive_system);
    schedule.add_systems(player::update_player_ping::handle);
}

pub fn register_schedules(
    timed: &mut Scheduler,
    shutdown_schedule: &mut Schedule,
    state: GlobalState,
) {
    let build_tick = |schedule: &mut Schedule| {
        schedule.set_executor(MultiThreadedExecutor::new());
        register_tick_systems(schedule);
    };
    let tick_period = Duration::from_secs(1) / state.config.tps;
    timed.register(
        TimedSchedule::new("tick", tick_period, build_tick)
            .with_behavior(MissedTickBehavior::Burst)
            .with_max_catch_up(5),
    );

    timed.register(
        TimedSchedule::new(
            "world_sync",
            Duration::from_secs(15),
            register_world_sync_schedule_systems,
        )
        .with_behavior(MissedTickBehavior::Skip),
    );

    timed.register(
        TimedSchedule::new(
            "chunk_gc",
            Duration::from_secs(5),
            register_chunk_gc_schedule_systems,
        )
        .with_behavior(MissedTickBehavior::Skip),
    );

    timed.register(
        TimedSchedule::new(
            "keepalive",
            Duration::from_secs(1),
            register_keepalive_schedule_systems,
        )
        .with_behavior(MissedTickBehavior::Skip)
        .with_phase(Duration::from_millis(250)),
    );
    shutdown_schedule.set_executor(SingleThreadedExecutor::new());

    // Force the chunk-saving systems to run before the world flushing and shutdown packet sending systems;
    // otherwise we might end up with a world not fully saved if the server is killed at the wrong time during shutdown
    shutdown_schedule.configure_sets(
        (
            ShutdownPhase::EmitSaveMessages,
            mobs::MobSaveSystems,
            ShutdownPhase::FlushWorld,
            ShutdownPhase::ShutdownPackets,
        )
            .chain(),
    );
    shutdown_schedule.add_systems(
        (
            shutdown::send_save_message::send_save_message,
            mobs::spawn::queue_live_mob_chunk_saves,
        )
            .chain()
            .in_set(ShutdownPhase::EmitSaveMessages),
    );
    mobs::register_save_systems(shutdown_schedule);
    shutdown_schedule
        .add_systems(background::world_sync::flush_world.in_set(ShutdownPhase::FlushWorld));
    shutdown_schedule
        .add_systems(shutdown::send_shutdown_packet::handle.in_set(ShutdownPhase::ShutdownPackets));

    for pending in drain_registered_schedules() {
        timed.register(pending.into_timed());
    }
}
