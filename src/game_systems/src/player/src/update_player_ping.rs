use bevy_ecs::prelude::Query;
use temper_components::player::keepalive::KeepAliveTracker;
use temper_components::player::player_identity::PlayerIdentity;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::player_info_update::PlayerInfoUpdatePacket;

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
