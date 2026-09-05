use bevy_ecs::prelude::{Commands, Entity, Has, MessageReader, MessageWriter, Query, Res, ResMut};
use temper_codec::net_types::length_prefixed_vec::LengthPrefixedVec;
use temper_components::bossbar::BossbarOwner;
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::position::Position;
use temper_core::dimension::Dimension::Overworld;
use temper_entities::MobKind;
use temper_messages::DespawnMob;
use temper_messages::destroy_entity::DestroyEntity;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::remove_entities::RemoveEntitiesPacket;
use temper_protocol::outgoing::system_message::SystemMessagePacket;
use temper_resources::bossbar::BossBarResource;
use temper_state::GlobalStateResource;
use temper_text::{Color, NamedColor, TextComponentBuilder};
use tracing::trace;

pub fn destroy_entity_system(
    mut commands: Commands,
    mut destroy_entity_events: MessageReader<DestroyEntity>,
    query: Query<(
        Entity,
        &Position,
        &Identity,
        &GameID,
        Has<PlayerMarker>,
        Has<MobKind>,
        Option<&StreamWriter>,
        Option<&BossbarOwner>,
    )>,
    state: Res<GlobalStateResource>,
    bossbar_res: ResMut<BossBarResource>,
    mut despawn_mobs: MessageWriter<DespawnMob>,
) {
    let mut destroyed_entities = Vec::new();
    let killed_message = SystemMessagePacket {
        message: temper_nbt::NBT::new(
            TextComponentBuilder::new("You have been killed. How sad :(")
                .bold()
                .color(Color::Named(NamedColor::Red))
                .build(),
        ),
        overlay: false,
    };

    for event in destroy_entity_events.read() {
        if let Ok((
            _,
            position,
            identity,
            game_id,
            has_player_marker,
            has_mob_kind,
            conn_opt,
            bossbar_own,
        )) = query.get(event.0)
        {
            if !has_player_marker {
                destroyed_entities.push(game_id.get());
                if has_mob_kind {
                    despawn_mobs.write(DespawnMob {
                        entity: event.0,
                        remove_from_chunk: true,
                    });
                    continue;
                }

                commands.entity(event.0).despawn();
                if let Some(owner) = bossbar_own {
                    bossbar_res.remove_bar(owner.id());
                }

                let Ok(chunk) = state.0.world.get_chunk(position.chunk(), Overworld) else {
                    continue;
                };
                if chunk.entities.remove(&identity.uuid).is_some() {
                    trace!(
                        "Entity {:?} destroyed and removed from chunk",
                        identity.uuid
                    );
                    chunk.mark_dirty();
                }
            } else if let Some(conn) = conn_opt
                && let Err(err) = conn.send_packet_ref(&killed_message)
            {
                trace!("Failed to send killed message: {}", err);
            }
        }
    }

    let packet = RemoveEntitiesPacket {
        entity_ids: LengthPrefixedVec::new(destroyed_entities),
    };

    for (_, _, _, _, has_player_marker, _, conn_opt, _) in query.iter() {
        if has_player_marker
            && let Some(conn) = conn_opt
            && let Err(err) = conn.send_packet_ref(&packet)
        {
            trace!("Failed to send RemoveEntitiesPacket: {}", err);
        }
    }
}
