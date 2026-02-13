use crate::systems::new_connections::NewConnectionRecv;
use crate::tui::ServerCommandReceiver;
use bevy_ecs::prelude::World;
use crossbeam_channel::Receiver;
use temper_components::player::time::WorldTime;
use temper_config::server_config::get_global_config;
use temper_core::chunks::world_sync_tracker::WorldSyncTracker;
use temper_net::connection::NewConnection;
use temper_performance::ServerPerformance;
use temper_state::GlobalStateResource;

pub fn register_resources(
    world: &mut World,
    new_conn_recv: Receiver<NewConnection>,
    global_state: GlobalStateResource,
    server_command_recv: Receiver<String>,
) {
    world.insert_resource(NewConnectionRecv(new_conn_recv));
    world.insert_resource(global_state);
    world.insert_resource(WorldSyncTracker {
        last_synced: std::time::Instant::now(),
    });
    world.insert_resource(WorldTime::default());
    world.insert_resource(ServerPerformance::new(get_global_config().tps));
    world.insert_resource(ServerCommandReceiver(server_command_recv));
}
