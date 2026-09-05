//! Handles PlayerInput packets for sneaking state changes.
//!
//! In 1.21.x protocol, sneaking is sent via PlayerInput packet (flag 0x20),
//! NOT via PlayerCommand (which was used in older protocol versions).

use bevy_ecs::prelude::{Entity, Query, Res};
use temper_codec::net_types::var_int::VarInt;
use temper_components::entity_identity::Identity;
use temper_components::player::entity_tracker::EntityTracker;
use temper_components::player::sneak::SneakState;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::PlayerInputReceiver;
use temper_protocol::outgoing::entity_metadata::{EntityMetadata, EntityMetadataPacket};
use tracing::{debug, warn};
use temper_components::game_id::GameID;

/// PlayerInput flags (1.21.x protocol)
const FLAG_SNEAK: u8 = 0x20;

/// Handles PlayerInput packets - specifically for sneaking state changes.
/// PlayerInput contains movement flags including sneak (0x20).
pub fn handle(
    receiver: Res<PlayerInputReceiver>,
    conn_query: Query<(Entity, &StreamWriter, &EntityTracker)>,
    identity_query: Query<(&Identity, &GameID)>,
    mut sneak_query: Query<&mut SneakState>,
) {
    for (event, eid) in receiver.0.try_iter() {
        let Ok((identity, game_id)) = identity_query.get(eid) else {
            continue;
        };

        // SneakState should always exist - it's part of PlayerBundle
        let Ok(mut sneak_state) = sneak_query.get_mut(eid) else {
            warn!(
                "SneakState component missing for player {} - this shouldn't happen",
                identity.name.as_ref().expect("No Player Name")
            );
            continue;
        };

        let is_sneaking = (event.flags & FLAG_SNEAK) != 0;

        // Only broadcast if state changed
        if is_sneaking == sneak_state.is_sneaking {
            continue;
        }

        sneak_state.is_sneaking = is_sneaking;

        debug!(
            "PlayerInput: sneak={} from {} (entity_id={})",
            is_sneaking,
            identity.name.as_ref().expect("No Player Name"),
            game_id.get()
        );

        let packet = if is_sneaking {
            EntityMetadataPacket::new(
                game_id.get(),
                [
                    EntityMetadata::entity_sneaking_flag(),
                    EntityMetadata::entity_sneaking_visual(),
                ],
            )
        } else {
            EntityMetadataPacket::new(
                game_id.get(),
                [
                    EntityMetadata::entity_clear_state(),
                    EntityMetadata::entity_standing(),
                ],
            )
        };

        for (recipient, writer, tracker) in conn_query.iter() {
            if recipient == eid || !writer.is_running() || !tracker.tracking.contains(&eid) {
                continue;
            }
            if let Err(err) = writer.send_packet_ref(&packet) {
                warn!("Failed to send player input metadata packet: {:?}", err);
            }
        }
    }
}
