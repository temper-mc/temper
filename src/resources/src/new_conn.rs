use bevy_ecs::prelude::Resource;
use crossbeam_channel::Receiver;
use temper_net_runtime::connection::NewConnection;

#[derive(Resource)]
pub struct NewConnectionRecv(pub Receiver<NewConnection>);
