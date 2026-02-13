use bevy_ecs::prelude::Resource;

#[derive(Resource)]
pub struct ServerCommandReceiver(pub crossbeam_channel::Receiver<String>);
