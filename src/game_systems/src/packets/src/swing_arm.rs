use bevy_ecs::prelude::{Entity, Query, Res};
use temper_codec::net_types::var_int::VarInt;
use temper_components::entity_identity::Identity;
use temper_components::player::entity_tracker::EntityTracker;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::SwingArmPacketReceiver;
use temper_protocol::outgoing::entity_animation::EntityAnimationPacket;
use temper_state::GlobalStateResource;
use tracing::error;
use temper_components::game_id::GameID;

pub fn handle(
    receiver: Res<SwingArmPacketReceiver>,
    query: Query<&GameID>,
    conn_query: Query<(Entity, &StreamWriter, &EntityTracker)>,
    state: Res<GlobalStateResource>,
) {
    for (event, eid) in receiver.0.try_iter() {
        let animation = { if event.hand == 0 { 0 } else { 3 } };
        let Ok(game_id) = query.get(eid) else {
            error!("Game ID not found for entity: {:?}", eid);
            continue;
        };
        let packet = EntityAnimationPacket::new(game_id.get(), animation);
        for (entity, conn, tracker) in conn_query.iter() {
            if entity == eid {
                continue; // Skip sending to the player who triggered the event
            }
            if !state.0.players.is_connected(entity) {
                continue; // Skip if the player is not connected
            }
            if !tracker.tracking.contains(&eid) {
                continue;
            }
            if let Err(e) = conn.send_packet_ref(&packet) {
                error!("Failed to send packet: {}", e);
            }
        }
    }
}
