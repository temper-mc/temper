use bevy_ecs::prelude::{Entity, Query, Res};
use ionic_components::player::player_identity::PlayerIdentity;
use ionic_net_runtime::connection::StreamWriter;
use ionic_state::GlobalStateResource;
use ionic_text::TextComponent;

pub fn handle(
    query: Query<(Entity, &StreamWriter, &PlayerIdentity)>,
    state: Res<GlobalStateResource>,
) {
    let packet = ionic_protocol::outgoing::disconnect::DisconnectPacket {
        reason: TextComponent::from("Server is shutting down").into(),
    };

    for (entity, conn, identity) in query.iter() {
        if state.0.players.is_connected(entity) {
            if let Err(e) = conn.send_packet_ref(&packet) {
                tracing::error!(
                    "Failed to send shutdown packet to player {}: {}",
                    identity.username,
                    e
                );
            } else {
                tracing::info!("Shutdown packet sent to player {}", identity.username);
            }
        }
    }
}
