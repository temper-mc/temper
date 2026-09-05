use bevy_ecs::prelude::*;
use mobs::spawn::{
    handle_spawn_mob_bundle, load_mob_bundles, queue_live_mob_chunk_saves, save_mob_bundles,
};
use shutdown::send_save_message::send_save_message;
use std::time::Instant;
use temper_components::entity_identity::Identity;
use temper_components::last_chunk_pos::LastChunkPos;
use temper_components::last_synced_position::LastSyncedPosition;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_core::dimension::Dimension;
use temper_core::pos::ChunkPos;
use temper_entities::entity_types::EntityTypeEnum;
use temper_entities::markers::entity_types::{Cow, Fox, Pig};
use temper_entities::markers::{HasCollisions, HasGravity, HasWaterDrag};
use temper_entities::{CowBundle, FoxBundle, MobBundle, MobKind, PigBundle};
use temper_messages::SpawnMobCommand;
use temper_messages::load_chunk_entities::LoadChunkEntities;
use temper_messages::save_chunk_entities::SaveChunkEntities;
use temper_resources::world_sync_tracker::WorldSyncTracker;
use temper_scheduler::Scheduler;
use temper_state::{GlobalStateResource, create_test_state};

fn emit_save_for(
    chunk: temper_core::pos::ChunkPos,
) -> impl FnMut(MessageWriter<SaveChunkEntities>) {
    move |mut writer: MessageWriter<SaveChunkEntities>| {
        writer.write(SaveChunkEntities(chunk));
    }
}

fn emit_load_for(
    chunk: temper_core::pos::ChunkPos,
) -> impl FnMut(MessageWriter<LoadChunkEntities>) {
    move |mut writer: MessageWriter<LoadChunkEntities>| {
        writer.write(LoadChunkEntities(chunk));
    }
}

fn emit_spawn_command(
    location: Position,
    entity_type: EntityTypeEnum,
) -> impl FnMut(MessageWriter<SpawnMobCommand>) {
    move |mut writer: MessageWriter<SpawnMobCommand>| {
        writer.write(SpawnMobCommand {
            entity_type,
            location,
        });
    }
}

fn ecs_world(state: GlobalStateResource) -> World {
    let mut world = World::new();
    temper_messages::register_messages(&mut world);
    world.insert_resource(state);
    world
}

fn ecs_world_with_sync(state: GlobalStateResource) -> World {
    let mut world = ecs_world(state);
    world.insert_resource(WorldSyncTracker {
        last_synced: Instant::now(),
    });
    world
}

fn spawn_pig(world: &mut World, bundle: PigBundle) -> Entity {
    let chunk = bundle.position.chunk();
    world
        .spawn((
            bundle,
            Pig,
            MobKind(EntityTypeEnum::Pig),
            HasGravity,
            HasCollisions,
            HasWaterDrag,
            LastChunkPos::new(chunk),
        ))
        .id()
}

fn save_live_mobs(world: &mut World) {
    let mut save_schedule = Schedule::default();
    save_schedule.add_systems((queue_live_mob_chunk_saves, save_mob_bundles).chain());
    save_schedule.run(world);
}

fn sync_and_clear_cache(state: &GlobalStateResource) {
    state
        .0
        .world
        .sync()
        .expect("saved mobs should be flushed to storage before restart-style load");
    state.0.world.get_cache().clear();
}

fn load_pigs_into_replacement_ecs(
    state: GlobalStateResource,
    chunk: ChunkPos,
) -> Vec<(Identity, Position)> {
    let mut world = ecs_world(state);
    let mut load_schedule = Schedule::default();
    load_schedule.add_systems(
        (
            emit_load_for(chunk),
            load_mob_bundles,
            handle_spawn_mob_bundle,
        )
            .chain(),
    );
    load_schedule.run(&mut world);

    let mut query = world.query::<(&Identity, &Position, Has<Pig>)>();
    query
        .iter(&world)
        .filter(|(_, _, is_pig)| *is_pig)
        .map(|(identity, position, _)| (identity.clone(), *position))
        .collect()
}

fn load_cows_into_replacement_ecs(
    state: GlobalStateResource,
    chunk: ChunkPos,
) -> Vec<(Identity, Position)> {
    let mut world = ecs_world(state);
    let mut load_schedule = Schedule::default();
    load_schedule.add_systems(
        (
            emit_load_for(chunk),
            load_mob_bundles,
            handle_spawn_mob_bundle,
        )
            .chain(),
    );
    load_schedule.run(&mut world);

    let mut query = world.query::<(&Identity, &Position, Has<Cow>)>();
    query
        .iter(&world)
        .filter(|(_, _, is_cow)| *is_cow)
        .map(|(identity, position, _)| (identity.clone(), *position))
        .collect()
}

fn run_registered_shutdown_schedule(world: &mut World) {
    let mut timed = Scheduler::new();
    let mut shutdown_schedule = Schedule::default();
    let state = world
        .get_resource::<GlobalStateResource>()
        .expect("world should have a state resource")
        .clone();
    temper_game_systems::register_schedules(&mut timed, &mut shutdown_schedule, state.0);
    shutdown_schedule.run(world);
}

#[test]
fn pig_round_trips_through_chunk_save_and_load() {
    let mut world = World::new();
    temper_messages::register_messages(&mut world);

    let (state, _temp_dir) = create_test_state();
    world.insert_resource(state);

    let position = Position::new(5.5, 64.0, 7.5);
    let chunk = position.chunk();
    let bundle = PigBundle::new(position);
    let expected_identity = bundle.identity.clone();
    let expected_last_synced = bundle.last_synced_position;

    let original_entity = world
        .spawn((
            bundle,
            Pig,
            MobKind(EntityTypeEnum::Pig),
            HasGravity,
            HasCollisions,
            HasWaterDrag,
        ))
        .id();

    let mut save_schedule = Schedule::default();
    save_schedule.add_systems((emit_save_for(chunk), save_mob_bundles).chain());
    save_schedule.run(&mut world);

    {
        let state = world.resource::<temper_state::GlobalStateResource>();
        let saved_chunk = state
            .0
            .world
            .get_chunk(chunk, Dimension::Overworld)
            .expect("chunk should exist after save");
        let saved_entity = saved_chunk
            .entities
            .get(&expected_identity.uuid)
            .expect("saved pig should be present in chunk storage");

        assert_eq!(saved_entity.value().0, EntityTypeEnum::Pig);
    }

    world.despawn(original_entity);

    let mut load_schedule = Schedule::default();
    load_schedule.add_systems(
        (
            emit_load_for(chunk),
            load_mob_bundles,
            handle_spawn_mob_bundle,
        )
            .chain(),
    );
    load_schedule.run(&mut world);

    let mut query = world.query::<(
        &Identity,
        &Position,
        &LastChunkPos,
        &LastSyncedPosition,
        Has<Pig>,
        Has<HasGravity>,
        Has<HasCollisions>,
        Has<HasWaterDrag>,
    )>();

    let loaded: Vec<_> = query.iter(&world).collect();
    assert_eq!(
        loaded.len(),
        1,
        "exactly one pig should be loaded back into ECS"
    );

    let (
        identity,
        loaded_position,
        last_chunk,
        last_synced,
        is_pig,
        has_gravity,
        has_collisions,
        has_water_drag,
    ) = &loaded[0];

    assert!(is_pig, "loaded entity should have the Pig marker");
    assert!(has_gravity, "loaded pig should regain HasGravity");
    assert!(has_collisions, "loaded pig should regain HasCollisions");
    assert!(has_water_drag, "loaded pig should regain HasWaterDrag");
    assert_eq!(identity.uuid, expected_identity.uuid);
    assert_eq!(loaded_position.coords, position.coords);
    assert_eq!(last_chunk.0, chunk);
    assert_eq!(last_synced.0, expected_last_synced.0);
}

#[test]
fn chunk_storage_does_not_persist_game_id() {
    let mut world = World::new();
    temper_messages::register_messages(&mut world);

    let (state, _temp_dir) = create_test_state();
    world.insert_resource(state);

    let position = Position::new(5.5, 64.0, 7.5);
    let chunk = position.chunk();
    let bundle = PigBundle::new(position);
    let expected_identity = bundle.identity.clone();
    let original_game_id = bundle.game_id;

    spawn_pig(&mut world, bundle);

    let mut save_schedule = Schedule::default();
    save_schedule.add_systems((emit_save_for(chunk), save_mob_bundles).chain());
    save_schedule.run(&mut world);

    let state = world.resource::<temper_state::GlobalStateResource>();
    let saved_chunk = state
        .0
        .world
        .get_chunk(chunk, Dimension::Overworld)
        .expect("chunk should exist after save");
    let saved_entity = saved_chunk
        .entities
        .get(&expected_identity.uuid)
        .expect("saved pig should be present in chunk storage");
    let saved_bundle = MobBundle::deserialize(saved_entity.value().0, &saved_entity.value().1)
        .expect("saved pig bundle should deserialize");

    let MobBundle::Pig(saved_pig) = saved_bundle else {
        panic!("saved entity should deserialize as a pig");
    };

    assert_eq!(saved_pig.identity.uuid, expected_identity.uuid);
    assert_ne!(saved_pig.game_id, original_game_id);
}

#[test]
fn shutdown_save_message_saves_spawned_pig_from_ecs() {
    let mut world = World::new();
    temper_messages::register_messages(&mut world);

    let (state, _temp_dir) = create_test_state();
    world.insert_resource(state);

    let position = Position::new(6.5, 64.0, 7.5);
    let chunk = position.chunk();
    let bundle = PigBundle::new(position);
    let expected_identity = bundle.identity.clone();

    {
        let state = world.resource::<temper_state::GlobalStateResource>();
        state
            .0
            .world
            .get_or_generate_chunk(chunk, Dimension::Overworld)
            .expect("chunk should be cached before shutdown save");
    }

    world.spawn((
        bundle,
        Pig,
        MobKind(EntityTypeEnum::Pig),
        HasGravity,
        HasCollisions,
        HasWaterDrag,
    ));

    let mut save_schedule = Schedule::default();
    save_schedule.add_systems((send_save_message, save_mob_bundles).chain());
    save_schedule.run(&mut world);

    let state = world.resource::<temper_state::GlobalStateResource>();
    let saved_chunk = state
        .0
        .world
        .get_chunk(chunk, Dimension::Overworld)
        .expect("chunk should exist after shutdown save");
    let saved_entity = saved_chunk
        .entities
        .get(&expected_identity.uuid)
        .expect("spawned pig should be saved by shutdown save messages");

    assert_eq!(saved_entity.value().0, EntityTypeEnum::Pig);
}

#[test]
fn spawn_command_cow_survives_registered_shutdown_reload() {
    let (state, _temp_dir) = create_test_state();
    let player_position = Position::new(6.5, 64.0, 7.5);
    let expected_position = player_position.offset_forward(&Rotation::new(0.0, 0.0), 2.0);
    let chunk = expected_position.chunk();

    let expected_identity = {
        let mut first_world = ecs_world_with_sync(state.clone());
        let mut spawn_schedule = Schedule::default();
        spawn_schedule.add_systems(
            (
                emit_spawn_command(expected_position, EntityTypeEnum::Cow),
                player::entity_spawn::spawn_command_processor,
                handle_spawn_mob_bundle,
            )
                .chain(),
        );
        spawn_schedule.run(&mut first_world);

        let mut cows = first_world.query::<(&Identity, &Position, Has<Cow>)>();
        let cows = cows
            .iter(&first_world)
            .filter(|(_, _, is_cow)| *is_cow)
            .map(|(identity, position, _)| (identity.clone(), *position))
            .collect::<Vec<_>>();

        assert_eq!(cows.len(), 1, "spawn command should create one cow");
        assert_eq!(cows[0].1.coords, expected_position.coords);

        run_registered_shutdown_schedule(&mut first_world);
        cows[0].0.clone()
    };

    state.0.world.get_cache().clear();
    let loaded = load_cows_into_replacement_ecs(state.clone(), chunk);

    assert_eq!(loaded.len(), 1, "spawned cow should reload after restart");
    assert_eq!(loaded[0].0.uuid, expected_identity.uuid);
    assert_eq!(loaded[0].1.coords, expected_position.coords);
}

#[test]
fn live_mob_chunk_save_saves_pig_when_chunk_is_not_cached() {
    let (state, _temp_dir) = create_test_state();
    let mut world = ecs_world(state);

    let position = Position::new(40.5, 64.0, 40.5);
    let chunk = position.chunk();
    let bundle = PigBundle::new(position);
    let expected_identity = bundle.identity.clone();

    spawn_pig(&mut world, bundle);
    save_live_mobs(&mut world);

    let state = world.resource::<temper_state::GlobalStateResource>();
    let saved_chunk = state
        .0
        .world
        .get_chunk(chunk, Dimension::Overworld)
        .expect("live mob chunk save should generate the pig chunk");
    let saved_entity = saved_chunk
        .entities
        .get(&expected_identity.uuid)
        .expect("pig should be saved even when its chunk was not already cached");

    assert_eq!(saved_entity.value().0, EntityTypeEnum::Pig);
}

#[test]
fn live_mob_chunk_save_keeps_clean_chunk_clean_when_mob_is_unchanged() {
    let (state, _temp_dir) = create_test_state();
    let mut world = ecs_world(state.clone());

    let position = Position::new(40.5, 64.0, 40.5);
    let chunk = position.chunk();
    spawn_pig(&mut world, PigBundle::new(position));

    save_live_mobs(&mut world);
    let saved_chunk = state
        .0
        .world
        .get_chunk(chunk, Dimension::Overworld)
        .expect("pig chunk should exist after first save");
    assert!(
        saved_chunk.is_dirty(),
        "first mob save should dirty the generated chunk"
    );

    saved_chunk.clear_dirty();
    save_live_mobs(&mut world);

    assert!(
        !saved_chunk.is_dirty(),
        "unchanged mob save should leave the clean chunk clean"
    );
}

#[test]
fn live_mob_chunk_save_marks_chunk_dirty_when_mob_changes() {
    let (state, _temp_dir) = create_test_state();
    let mut world = ecs_world(state.clone());

    let initial_position = Position::new(40.5, 64.0, 40.5);
    let moved_position = Position::new(41.5, 64.0, 40.5);
    let chunk = initial_position.chunk();
    let pig = spawn_pig(&mut world, PigBundle::new(initial_position));

    save_live_mobs(&mut world);
    let saved_chunk = state
        .0
        .world
        .get_chunk(chunk, Dimension::Overworld)
        .expect("pig chunk should exist after first save");
    saved_chunk.clear_dirty();

    {
        let mut position = world
            .get_mut::<Position>(pig)
            .expect("pig should still be alive");
        *position = moved_position;
    }

    save_live_mobs(&mut world);

    assert!(
        saved_chunk.is_dirty(),
        "changed mob save should mark the chunk dirty"
    );
    let saved_entity = saved_chunk
        .entities
        .iter()
        .next()
        .expect("pig should remain saved");
    let saved_bundle = MobBundle::deserialize(saved_entity.value().0, &saved_entity.value().1)
        .expect("saved pig bundle should deserialize");
    assert_eq!(saved_bundle.position().coords, moved_position.coords);
}

#[test]
fn live_mob_chunk_save_removes_same_mob_from_old_cached_chunk() {
    let (state, _temp_dir) = create_test_state();
    let mut world = ecs_world(state.clone());

    let old_position = Position::new(40.5, 64.0, 40.5);
    let new_position = Position::new(57.5, 64.0, 40.5);
    let old_chunk_pos = old_position.chunk();
    let new_chunk_pos = new_position.chunk();
    let bundle = PigBundle::new(old_position);
    let expected_identity = bundle.identity.clone();
    let pig = spawn_pig(&mut world, bundle);

    save_live_mobs(&mut world);
    {
        let old_chunk = state
            .0
            .world
            .get_chunk(old_chunk_pos, Dimension::Overworld)
            .expect("old pig chunk should exist");
        assert!(old_chunk.entities.contains_key(&expected_identity.uuid));
        old_chunk.clear_dirty();
    }

    {
        let mut position = world
            .get_mut::<Position>(pig)
            .expect("pig should still be alive");
        *position = new_position;
    }

    save_live_mobs(&mut world);

    let old_chunk = state
        .0
        .world
        .get_chunk(old_chunk_pos, Dimension::Overworld)
        .expect("old pig chunk should still exist");
    assert!(
        !old_chunk.entities.contains_key(&expected_identity.uuid),
        "moving mob save should clear stale chunk entry"
    );
    assert!(old_chunk.is_dirty());

    let new_chunk = state
        .0
        .world
        .get_chunk(new_chunk_pos, Dimension::Overworld)
        .expect("new pig chunk should exist");
    let saved_entity = new_chunk
        .entities
        .get(&expected_identity.uuid)
        .expect("moving mob should be saved in its new chunk");
    let saved_bundle = MobBundle::deserialize(saved_entity.value().0, &saved_entity.value().1)
        .expect("saved pig bundle should deserialize");
    assert_eq!(saved_bundle.position().coords, new_position.coords);
}

#[test]
fn mob_load_removes_entry_whose_position_belongs_to_another_chunk() {
    let (state, _temp_dir) = create_test_state();

    let wrong_chunk_pos = ChunkPos::new(0, 0);
    let real_position = Position::new(40.5, 64.0, 40.5);
    let bundle = PigBundle::new(real_position);
    let stale_uuid = bundle.identity.uuid;
    {
        let wrong_chunk = state
            .0
            .world
            .get_or_generate_chunk(wrong_chunk_pos, Dimension::Overworld)
            .expect("wrong chunk should be cached");
        wrong_chunk.entities.insert(
            stale_uuid,
            (
                EntityTypeEnum::Pig,
                MobBundle::Pig(bundle).serialize_for_chunk(),
            ),
        );
        wrong_chunk.clear_dirty();
    }

    let loaded = load_pigs_into_replacement_ecs(state.clone(), wrong_chunk_pos);
    assert!(
        loaded.is_empty(),
        "stale mob entry should not spawn from the wrong chunk"
    );

    let wrong_chunk = state
        .0
        .world
        .get_chunk(wrong_chunk_pos, Dimension::Overworld)
        .expect("wrong chunk should still exist");
    assert!(!wrong_chunk.entities.contains_key(&stale_uuid));
    assert!(wrong_chunk.is_dirty());
}

#[test]
fn mob_load_skips_mob_that_is_already_live() {
    let (state, _temp_dir) = create_test_state();
    let mut world = ecs_world(state.clone());

    let position = Position::new(40.5, 64.0, 40.5);
    let chunk_pos = position.chunk();
    let persisted = MobBundle::Pig(PigBundle::new(position));
    let expected_uuid = persisted.identity().uuid;

    {
        let chunk = state
            .0
            .world
            .get_or_generate_chunk(chunk_pos, Dimension::Overworld)
            .expect("pig chunk should be cached");
        chunk.entities.insert(
            expected_uuid,
            (EntityTypeEnum::Pig, persisted.serialize_for_chunk()),
        );
    }

    let MobBundle::Pig(live_bundle) = persisted.clone() else {
        unreachable!("test bundle is a pig");
    };
    spawn_pig(&mut world, live_bundle);

    let mut load_schedule = Schedule::default();
    load_schedule.add_systems(
        (
            emit_load_for(chunk_pos),
            load_mob_bundles,
            handle_spawn_mob_bundle,
        )
            .chain(),
    );
    load_schedule.run(&mut world);

    let mut pigs = world.query::<(&Identity, Has<Pig>)>();
    let live_pigs = pigs
        .iter(&world)
        .filter(|(_, is_pig)| *is_pig)
        .map(|(identity, _)| identity.uuid)
        .collect::<Vec<_>>();

    assert_eq!(live_pigs, vec![expected_uuid]);
}

#[test]
fn live_mob_chunk_save_flushes_uncached_pig_to_storage() {
    let (state, _temp_dir) = create_test_state();

    let position = Position::new(40.5, 64.0, 40.5);
    let chunk = position.chunk();
    let bundle = PigBundle::new(position);
    let expected_identity = bundle.identity.clone();

    {
        let mut first_world = ecs_world_with_sync(state.clone());
        spawn_pig(&mut first_world, bundle);
        save_live_mobs(&mut first_world);
    }

    sync_and_clear_cache(&state);
    let loaded = load_pigs_into_replacement_ecs(state.clone(), chunk);

    assert_eq!(
        loaded.len(),
        1,
        "exactly one pig should reload after flushing storage"
    );

    let (identity, loaded_position) = &loaded[0];
    assert_eq!(identity.uuid, expected_identity.uuid);
    assert_eq!(loaded_position.coords, position.coords);
}

#[test]
fn registered_shutdown_schedule_saves_pig_into_replacement_ecs_world() {
    let (state, _temp_dir) = create_test_state();

    let position = Position::new(45.5, 64.0, 45.5);
    let chunk = position.chunk();
    let bundle = PigBundle::new(position);
    let expected_identity = bundle.identity.clone();

    {
        let mut first_world = ecs_world_with_sync(state.clone());
        spawn_pig(&mut first_world, bundle);
        run_registered_shutdown_schedule(&mut first_world);
    }

    state.0.world.get_cache().clear();
    let loaded = load_pigs_into_replacement_ecs(state.clone(), chunk);

    assert_eq!(
        loaded.len(),
        1,
        "registered shutdown schedule should persist one pig for replacement ECS load"
    );

    let (identity, loaded_position) = &loaded[0];
    assert_eq!(identity.uuid, expected_identity.uuid);
    assert_eq!(loaded_position.coords, position.coords);
}

#[test]
fn mob_save_refreshes_existing_chunk_entry_with_live_position() {
    let mut world = World::new();
    temper_messages::register_messages(&mut world);

    let (state, _temp_dir) = create_test_state();
    world.insert_resource(state.clone());

    let spawn_position = Position::new(6.5, 64.0, 7.5);
    let moved_position = Position::new(8.5, 64.0, 7.5);
    let chunk = spawn_position.chunk();
    let cow_bundle = CowBundle::new(spawn_position);
    let expected_identity = cow_bundle.identity.clone();

    {
        let chunk = state
            .0
            .world
            .get_or_generate_chunk(chunk, Dimension::Overworld)
            .expect("chunk should be cached before save");
        chunk.entities.insert(
            expected_identity.uuid,
            (
                EntityTypeEnum::Cow,
                MobBundle::Cow(CowBundle::new(spawn_position)).serialize_for_chunk(),
            ),
        );
        chunk.mark_dirty();
    }

    let cow_entity = world
        .spawn((
            cow_bundle,
            Cow,
            MobKind(EntityTypeEnum::Cow),
            HasGravity,
            HasCollisions,
            HasWaterDrag,
            LastChunkPos::new(chunk),
        ))
        .id();

    {
        let mut position = world
            .get_mut::<Position>(cow_entity)
            .expect("cow should still be alive");
        *position = moved_position;
    }

    let mut save_schedule = Schedule::default();
    save_schedule.add_systems((emit_save_for(chunk), save_mob_bundles).chain());
    save_schedule.run(&mut world);

    let state = world.resource::<temper_state::GlobalStateResource>();
    let saved_chunk = state
        .0
        .world
        .get_chunk(chunk, Dimension::Overworld)
        .expect("chunk should exist after save");
    let saved_entity = saved_chunk
        .entities
        .get(&expected_identity.uuid)
        .expect("moved cow should still be stored");
    let saved_bundle = MobBundle::deserialize(saved_entity.value().0, &saved_entity.value().1)
        .expect("saved cow bundle should deserialize");

    assert_eq!(saved_bundle.position().coords, moved_position.coords);
}

#[test]
fn fox_loads_in_a_separate_ecs_world_after_save() {
    let (state, _temp_dir) = create_test_state();

    let position = Position::new(23.5, 70.0, -10.25);
    let chunk = position.chunk();
    let bundle = FoxBundle::new(position);
    let expected_identity = bundle.identity.clone();
    let expected_last_synced = bundle.last_synced_position;

    {
        let mut first_world = World::new();
        temper_messages::register_messages(&mut first_world);
        first_world.insert_resource(state.clone());
        first_world.spawn((
            bundle,
            Fox,
            MobKind(EntityTypeEnum::Fox),
            HasGravity,
            HasCollisions,
            HasWaterDrag,
        ));

        let mut save_schedule = Schedule::default();
        save_schedule.add_systems((emit_save_for(chunk), save_mob_bundles).chain());
        save_schedule.run(&mut first_world);
    }

    state
        .0
        .world
        .sync()
        .expect("saved fox should be flushed to storage before restart-style load");
    state.0.world.get_cache().clear();

    let loaded = {
        let mut second_world = World::new();
        temper_messages::register_messages(&mut second_world);
        second_world.insert_resource(state.clone());

        let mut load_schedule = Schedule::default();
        load_schedule.add_systems(
            (
                emit_load_for(chunk),
                load_mob_bundles,
                handle_spawn_mob_bundle,
            )
                .chain(),
        );
        load_schedule.run(&mut second_world);

        let mut query = second_world.query::<(
            &Identity,
            &Position,
            &LastChunkPos,
            &LastSyncedPosition,
            Has<Fox>,
            Has<HasGravity>,
            Has<HasCollisions>,
            Has<HasWaterDrag>,
        )>();

        query
            .iter(&second_world)
            .map(
                |(
                    identity,
                    loaded_position,
                    last_chunk,
                    last_synced,
                    is_fox,
                    has_gravity,
                    has_collisions,
                    has_water_drag,
                )| {
                    (
                        identity.clone(),
                        *loaded_position,
                        *last_chunk,
                        *last_synced,
                        is_fox,
                        has_gravity,
                        has_collisions,
                        has_water_drag,
                    )
                },
            )
            .collect::<Vec<_>>()
    };

    assert_eq!(
        loaded.len(),
        1,
        "exactly one fox should be loaded into the replacement ECS world"
    );

    let (
        identity,
        loaded_position,
        last_chunk,
        last_synced,
        is_fox,
        has_gravity,
        has_collisions,
        has_water_drag,
    ) = &loaded[0];

    assert!(is_fox, "loaded entity should have the Fox marker");
    assert!(has_gravity, "loaded fox should regain HasGravity");
    assert!(has_collisions, "loaded fox should regain HasCollisions");
    assert!(has_water_drag, "loaded fox should regain HasWaterDrag");
    assert_eq!(identity.uuid, expected_identity.uuid);
    assert_eq!(loaded_position.coords, position.coords);
    assert_eq!(last_chunk.0, chunk);
    assert_eq!(last_synced.0, expected_last_synced.0);
}
