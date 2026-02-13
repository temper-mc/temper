use bevy_ecs::prelude::Query;
use ionic_components::player::keepalive::KeepAliveTracker;
use ionic_components::player::player_identity::PlayerIdentity;
use ionic_net_runtime::connection::StreamWriter;
use ionic_protocol::outgoing::player_info_update::PlayerInfoUpdatePacket;

pub fn handle(query: Query<(&PlayerIdentity, &KeepAliveTracker)>, conns: Query<&StreamWriter>) {
    let packet = PlayerInfoUpdatePacket::update_players_ping(
        query
            .iter()
            .map(|(identity, keepalive)| {
                (identity.uuid.as_u128(), (keepalive.ping() as i32).into())
            })
            .collect(),
    );
    for conn in conns.iter() {
        if let Err(err) = conn.send_packet_ref(&packet) {
            tracing::warn!("Failed to send player ping update packet: {:?}", err);
        }
    }
}
