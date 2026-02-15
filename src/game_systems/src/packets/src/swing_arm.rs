use bevy_ecs::prelude::{Entity, Query, Res};
use temper_codec::net_types::var_int::VarInt;
use temper_components::player::player_identity::PlayerIdentity;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::SwingArmPacketReceiver;
use temper_protocol::outgoing::entity_animation::EntityAnimationPacket;
use temper_state::GlobalStateResource;
use tracing::error;

pub fn handle(
    receiver: Res<SwingArmPacketReceiver>,
    query: Query<&PlayerIdentity>,
    conn_query: Query<(Entity, &StreamWriter)>,
    state: Res<GlobalStateResource>,
) {
    for (event, eid) in receiver.0.try_iter() {
        let animation = { if event.hand == 0 { 0 } else { 3 } };
        let game_id = query.get(eid).expect("Game ID not found");
        let packet = EntityAnimationPacket::new(VarInt::new(game_id.short_uuid), animation);
        for (entity, conn) in conn_query.iter() {
            if entity == eid {
                continue; // Skip sending to the player who triggered the event
            }
            if !state.0.players.is_connected(entity) {
                continue; // Skip if the player is not connected
            }
            if let Err(e) = conn.send_packet_ref(&packet) {
                error!("Failed to send packet: {}", e);
            }
        }
    }
}
