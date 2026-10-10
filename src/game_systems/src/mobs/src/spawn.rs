use bevy_ecs::prelude::*;
use std::collections::HashSet;
use temper_components::bossbar::BossbarOwner;
use temper_components::combat::CombatProperties;
use temper_components::entity_identity::Identity;
use temper_components::health::Health;
use temper_components::last_synced_position::LastSyncedPosition;
use temper_components::metadata::EntityMetadata;
use temper_components::player::entity_tracker::EntityTracker;
use temper_components::player::grounded::OnGround;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_components::player::velocity::Velocity;
use temper_components::spawn::SpawnProperties;
use temper_core::dimension::Dimension;
use temper_entities::mob_definition::StandardMobParts;
use temper_entities::{MobBundle, MobKind};
use temper_messages::{
    DespawnMob, SpawnMobBundle, load_chunk_entities::LoadChunkEntities,
    save_chunk_entities::SaveChunkEntities,
};
use temper_resources::bossbar::BossBarResource;
use temper_state::GlobalStateResource;

type StandardMobQuery<'a> = (
    &'a Identity,
    &'a EntityMetadata,
    &'a CombatProperties,
    &'a SpawnProperties,
    &'a Position,
    &'a Rotation,
    &'a Velocity,
    &'a OnGround,
    &'a LastSyncedPosition,
    &'a MobKind,
);

pub fn handle_spawn_mob_bundle(
    mut events: MessageReader<SpawnMobBundle>,
    mut commands: Commands,
    state: Res<GlobalStateResource>,
    query: Query<&EntityTracker>,
) {
    for event in events.read() {
        let kind = event.bundle.kind();
        let uuid = event.bundle.identity().uuid;
        let position = event.bundle.position();

        if event.persist {
            let chunk = state
                .0
                .world
                .get_or_generate_chunk(position.chunk(), Dimension::Overworld)
                .expect("Failed to get or generate chunk");
            chunk
                .entities
                .insert(uuid, (kind, event.bundle.serialize_for_chunk()));
            chunk.mark_dirty();
            tracing::trace!(
                "Persisted spawned {:?} mob {:?} at {} in chunk {}",
                kind,
                uuid,
                position,
                position.chunk()
            );
        }

        let mob_entity = event.bundle.clone().spawn_standard(&mut commands);

        let entity_health_opt = kind.to_entity_type().get_attribute("max_health");

        let mut health = Health::default();

        if let Some(entity_health) = entity_health_opt {
            health.max = entity_health as f32;
            health.current = health.max;
        }

        commands.entity(mob_entity).insert(health);

        query.iter().for_each(|tracker| {
            tracker.to_track.push((uuid, kind.to_entity_type().id));
        });
    }
}

pub fn handle_despawn_mob(
    mut commands: Commands,
    mut events: MessageReader<DespawnMob>,
    query: Query<(&Position, &Identity, Option<&BossbarOwner>), With<MobKind>>,
    trackers: Query<&EntityTracker>,
    state: Res<GlobalStateResource>,
    bossbar_res: Option<Res<BossBarResource>>,
) {
    let mut despawned = HashSet::new();

    for event in events.read() {
        if !despawned.insert(event.entity) {
            continue;
        }

        let Ok((position, identity, bossbar_owner)) = query.get(event.entity) else {
            continue;
        };

        if event.remove_from_chunk
            && let Ok(chunk) = state
                .0
                .world
                .get_chunk(position.chunk(), Dimension::Overworld)
            && chunk.entities.remove(&identity.uuid).is_some()
        {
            chunk.mark_dirty();
        }

        if let (Some(owner), Some(bossbar_res)) = (bossbar_owner, bossbar_res.as_ref()) {
            bossbar_res.remove_bar(owner.id());
        }

        commands.entity(event.entity).despawn();

        for tracker in trackers.iter() {
            tracker.to_untrack.push(event.entity);
        }
    }
}

pub fn load_mob_bundles(
    state: Res<GlobalStateResource>,
    mut load_events: MessageReader<LoadChunkEntities>,
    mut spawn_events: MessageWriter<SpawnMobBundle>,
    live_mobs: Query<&Identity, With<MobKind>>,
) {
    let live_mobs = live_mobs
        .iter()
        .map(|identity| identity.uuid)
        .collect::<HashSet<_>>();
    let mut loaded_chunks = HashSet::new();

    for event in load_events.read() {
        if !loaded_chunks.insert(event.0) {
            continue;
        }

        let Ok(chunk) = state.0.world.get_chunk(event.0, Dimension::Overworld) else {
            tracing::error!("Failed to load chunk {} for entity loading", event.0);
            continue;
        };

        let mut stale_entries = Vec::new();
        for kv in chunk.entities.iter() {
            let (kind, data) = kv.value();
            let Some(bundle) = MobBundle::deserialize(*kind, data) else {
                tracing::error!(
                    "Failed to deserialize persisted {:?} mob {:?} in chunk {}",
                    kind,
                    kv.key(),
                    event.0
                );
                continue;
            };

            if bundle.position().chunk() != event.0 {
                tracing::warn!(
                    "Removing persisted {:?} mob {:?} from chunk {}; position belongs to chunk {}",
                    kind,
                    kv.key(),
                    event.0,
                    bundle.position().chunk()
                );
                stale_entries.push(*kv.key());
                continue;
            }

            if live_mobs.contains(kv.key()) {
                continue;
            }

            spawn_events.write(SpawnMobBundle {
                bundle,
                persist: false,
            });
        }

        for uuid in stale_entries {
            if chunk.entities.remove(&uuid).is_some() {
                chunk.mark_dirty();
            }
        }
    }
}

pub fn save_mob_bundles(
    state: Res<GlobalStateResource>,
    query: Query<StandardMobQuery>,
    mut save_events: MessageReader<SaveChunkEntities>,
) {
    let mut saved_chunks = HashSet::new();

    let deduped = save_events.read().collect::<HashSet<_>>();

    for event in deduped {
        if !saved_chunks.insert(event.0) {
            continue;
        }

        let mut saved_mobs = 0;

        for (
            identity,
            metadata,
            combat,
            spawn,
            position,
            rotation,
            velocity,
            on_ground,
            last_synced_position,
            mob_kind,
        ) in query.iter()
        {
            if position.chunk() != event.0 {
                continue;
            }

            let bundle = standard_mob_bundle(
                identity,
                metadata,
                combat,
                spawn,
                position,
                rotation,
                velocity,
                on_ground,
                last_synced_position,
                mob_kind,
            );

            let kind = bundle.kind();
            let uuid = bundle.identity().uuid;
            let data = bundle.serialize_for_chunk();

            let changed = {
                let chunk = state
                    .0
                    .world
                    .get_or_generate_chunk(event.0, Dimension::Overworld)
                    .expect("Failed to get or generate chunk");

                let unchanged = chunk.entities.get(&uuid).is_some_and(|stored| {
                    let (stored_kind, stored_data) = stored.value();
                    *stored_kind == kind && stored_data.as_slice() == data.as_slice()
                });
                if unchanged {
                    false
                } else {
                    chunk.entities.insert(uuid, (kind, data));
                    chunk.mark_dirty();
                    true
                }
            };

            if !changed {
                continue;
            }
            remove_stale_mob_entries(&state, uuid, event.0);

            saved_mobs += 1;
        }

        if saved_mobs > 0 {
            tracing::trace!("Saved {} mob(s) in chunk {}", saved_mobs, event.0);
        }
    }
}

pub fn queue_live_mob_chunk_saves(
    query: Query<&Position, With<MobKind>>,
    mut save_events: MessageWriter<SaveChunkEntities>,
) {
    let mut chunks = HashSet::new();

    for position in query.iter() {
        let chunk = position.chunk();
        if chunks.insert(chunk) {
            save_events.write(SaveChunkEntities(chunk));
        }
    }
}

fn remove_stale_mob_entries(
    state: &GlobalStateResource,
    uuid: uuid::Uuid,
    current_chunk: temper_core::pos::ChunkPos,
) {
    for entry in state.0.world.get_cache() {
        let ((chunk_pos, _), chunk) = entry.pair();
        if *chunk_pos == current_chunk {
            continue;
        }

        if chunk.entities.remove(&uuid).is_some() {
            chunk.mark_dirty();
        }
    }
}

fn standard_mob_bundle(
    identity: &Identity,
    metadata: &EntityMetadata,
    combat: &CombatProperties,
    spawn: &SpawnProperties,
    position: &Position,
    rotation: &Rotation,
    velocity: &Velocity,
    on_ground: &OnGround,
    last_synced_position: &LastSyncedPosition,
    mob_kind: &MobKind,
) -> MobBundle {
    MobBundle::from_standard_parts(
        mob_kind.0,
        StandardMobParts {
            identity,
            metadata,
            combat,
            spawn,
            position,
            rotation,
            velocity,
            on_ground,
            last_synced_position,
        },
    )
}
