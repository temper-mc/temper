mod particles;

pub fn register_world_systems(schedule: &mut bevy_ecs::schedule::Schedule) {
    schedule.add_systems(particles::handle);
}
