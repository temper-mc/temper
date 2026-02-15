use bevy_ecs::prelude::Res;
use bevy_ecs::system::Query;
use std::time::Instant;
use temper_components::player::keepalive::KeepAliveTracker;
use temper_protocol::IncomingKeepAlivePacketReceiver;
use temper_state::GlobalStateResource;
use tracing::{error, warn};

pub fn handle(
    receiver: Res<IncomingKeepAlivePacketReceiver>,
    mut query: Query<&mut KeepAliveTracker>,
    state: Res<GlobalStateResource>,
) {
    for (event, eid) in receiver.0.try_iter() {
        let Ok(mut keep_alive_tracker) = query.get_mut(eid) else {
            error!("Could not get keep alive tracker for entity {:?}", eid);
            continue;
        };
        if event.timestamp != keep_alive_tracker.last_sent_keep_alive_id {
            warn!(
                "Invalid keep alive packet received from {:?} with id {:?} (expected {:?})",
                eid, event.timestamp, keep_alive_tracker.last_sent_keep_alive_id
            );
            state
                .0
                .players
                .disconnect(eid, Some("Invalid keep alive packet received".to_string()));
        } else {
            keep_alive_tracker.last_received_keep_alive = Instant::now();
            keep_alive_tracker.has_received_keep_alive = true;
        }
    }
}
