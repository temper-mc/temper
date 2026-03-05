pub mod chunk_sending;
pub mod chunk_unloader;
pub mod connection_killer;
pub mod day_cycle;
pub mod keep_alive_system;
pub mod lan_pinger;
pub mod mq;
pub mod send_entity_updates;
pub mod send_particles;
pub mod server_command;
pub mod world_sync;

pub fn register_background_systems(schedule: &mut bevy_ecs::prelude::Schedule) {
    schedule.add_systems(chunk_sending::handle);
    schedule.add_systems(connection_killer::connection_killer);
    schedule.add_systems(day_cycle::tick_daylight_cycle);
    schedule.add_systems(mq::process);
    schedule.add_systems(send_entity_updates::handle);
    schedule.add_systems(send_particles::handle);
    schedule.add_systems(server_command::handle);
}
