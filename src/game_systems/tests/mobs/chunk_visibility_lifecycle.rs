use crate::{clear_cached_chunks, emit_load_messages_for_known_chunks, wait_for_saved_chunk};
use background::{chunk_unloader, entity_unloader};
use bevy_ecs::prelude::*;
use mobs::spawn::{
    handle_despawn_mob, handle_spawn_mob_bundle, load_mob_bundles, save_mob_bundles,
};
use player::chunk_calculator;
use temper_components::entity_identity::Identity;
use temper_components::last_chunk_pos::LastChunkPos;
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_components::player::client_information::ClientInformationComponent;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::position::Position;
use temper_core::dimension::Dimension;
use temper_core::pos::FullChunkPos;
use temper_entities::MobKind;
use temper_entities::entity_types::EntityTypeEnum;
use temper_entities::markers::entity_types::{Fox, Pig};
use temper_entities::markers::{HasCollisions, HasGravity, HasWaterDrag};
use temper_entities::{FoxBundle, PigBundle};
use temper_messages::chunk_calc::ChunkCalc;
use temper_state::create_test_state;

fn emit_chunk_calc_for(entity: Entity) -> impl FnMut(MessageWriter<ChunkCalc>) {
    move |mut writer: MessageWriter<ChunkCalc>| {
        writer.write(ChunkCalc(entity));
    }
}

fn spawn_test_player(world: &mut World, position: Position, loaded_chunk: FullChunkPos) -> Entity {
    let mut receiver = ChunkReceiver::default();
    receiver
        .chunks
        .set_needed(&std::collections::HashSet::from([loaded_chunk]));
    assert!(receiver.chunks.mark_loaded(loaded_chunk));

    world
        .spawn((
            position,
            receiver,
            ClientInformationComponent {
                view_distance: 2,
                ..Default::default()
            },
            PlayerMarker,
        ))
        .id()
}

#[test]
fn multiple_entities_in_one_chunk_reload_together_when_player_returns() {
    let mut world = World::new();
    temper_messages::register_messages(&mut world);

    let (state, _temp_dir) = create_test_state();
    world.insert_resource(state.clone());

    let fox_position = Position::new(8.0, 64.0, 8.0, Dimension::Overworld);
    let pig_position = Position::new(9.0, 64.0, 9.0, Dimension::Overworld);
    let chunk = fox_position.chunk();

    let fox_bundle = FoxBundle::new(fox_position);
    let pig_bundle = PigBundle::new(pig_position);
    let expected_fox_uuid = fox_bundle.identity.uuid;
    let expected_pig_uuid = pig_bundle.identity.uuid;

    let player_entity =
        spawn_test_player(&mut world, fox_position, chunk.full(Dimension::Overworld));

    world.spawn((
        fox_bundle,
        Fox,
        MobKind(EntityTypeEnum::Fox),
        HasGravity,
        HasCollisions,
        HasWaterDrag,
        LastChunkPos::new(chunk.full(Dimension::Overworld)),
    ));
    world.spawn((
        pig_bundle,
        Pig,
        MobKind(EntityTypeEnum::Pig),
        HasGravity,
        HasCollisions,
        HasWaterDrag,
        LastChunkPos::new(chunk.full(Dimension::Overworld)),
    ));

    {
        let chunk_ref = state
            .0
            .world
            .get_or_generate_chunk(chunk.full(Dimension::Overworld))
            .expect("test chunk should be cached");
        chunk_ref.entities.clear();
        chunk_ref.mark_dirty();
    }

    {
        let mut player_pos = world
            .get_mut::<Position>(player_entity)
            .expect("player should exist");
        *player_pos = Position::new(2048.0, 64.0, 2048.0, Dimension::Overworld);
    }

    let mut chunk_calc_schedule = Schedule::default();
    chunk_calc_schedule
        .add_systems((emit_chunk_calc_for(player_entity), chunk_calculator::handle).chain());
    chunk_calc_schedule.run(&mut world);

    let mut unload_schedule = Schedule::default();
    unload_schedule.add_systems(
        (
            entity_unloader::handle,
            save_mob_bundles,
            chunk_unloader::handle,
            handle_despawn_mob,
        )
            .chain(),
    );
    unload_schedule.run(&mut world);

    let mut entity_ref_query = world.query::<EntityRef<'_>>();
    assert_eq!(
        entity_ref_query
            .iter(&world)
            .filter(|entity| entity.contains::<Fox>())
            .count(),
        0,
        "fox should unload when the player leaves the chunk"
    );
    assert_eq!(
        entity_ref_query
            .iter(&world)
            .filter(|entity| entity.contains::<Pig>())
            .count(),
        0,
        "pig should unload when the player leaves the chunk"
    );

    {
        let saved_chunk = wait_for_saved_chunk(&state, chunk.full(Dimension::Overworld));
        assert_eq!(
            saved_chunk.entities.len(),
            2,
            "both entities should be persisted"
        );
    }
    clear_cached_chunks(&state);

    {
        let mut receiver = world
            .get_mut::<ChunkReceiver>(player_entity)
            .expect("player chunk receiver should exist");
        receiver.chunks.reset();
    }
    {
        let mut player_pos = world
            .get_mut::<Position>(player_entity)
            .expect("player should still exist");
        *player_pos = fox_position;
    }

    chunk_calc_schedule.run(&mut world);

    let mut load_schedule = Schedule::default();
    load_schedule.add_systems(
        (
            emit_load_messages_for_known_chunks,
            load_mob_bundles,
            handle_spawn_mob_bundle,
        )
            .chain(),
    );
    load_schedule.run(&mut world);

    let mut fox_query = world.query::<(&Identity, Has<Fox>)>();
    let loaded_foxes: Vec<_> = fox_query
        .iter(&world)
        .filter(|(_, is_fox)| *is_fox)
        .map(|(identity, _)| identity.uuid)
        .collect();
    let mut pig_query = world.query::<(&Identity, Has<Pig>)>();
    let loaded_pigs: Vec<_> = pig_query
        .iter(&world)
        .filter(|(_, is_pig)| *is_pig)
        .map(|(identity, _)| identity.uuid)
        .collect();

    assert_eq!(loaded_foxes, vec![expected_fox_uuid]);
    assert_eq!(loaded_pigs, vec![expected_pig_uuid]);
}

#[test]
fn chunk_stays_loaded_while_a_second_player_keeps_it_visible() {
    let mut world = World::new();
    temper_messages::register_messages(&mut world);

    let (state, _temp_dir) = create_test_state();
    world.insert_resource(state.clone());

    let fox_position = Position::new(8.0, 64.0, 8.0, Dimension::Overworld);
    let chunk = fox_position.chunk();
    let fox_bundle = FoxBundle::new(fox_position);
    let fox_uuid = fox_bundle.identity.uuid;

    let moving_player =
        spawn_test_player(&mut world, fox_position, chunk.full(Dimension::Overworld));
    let _anchored_player =
        spawn_test_player(&mut world, fox_position, chunk.full(Dimension::Overworld));

    world.spawn((
        fox_bundle,
        Fox,
        MobKind(EntityTypeEnum::Fox),
        HasGravity,
        HasCollisions,
        HasWaterDrag,
        LastChunkPos::new(chunk.full(Dimension::Overworld)),
    ));

    {
        let chunk_ref = state
            .0
            .world
            .get_or_generate_chunk(chunk.full(Dimension::Overworld))
            .expect("test chunk should be cached");
        chunk_ref.entities.clear();
        chunk_ref.mark_dirty();
    }

    {
        let mut player_pos = world
            .get_mut::<Position>(moving_player)
            .expect("moving player should exist");
        *player_pos = Position::new(2048.0, 64.0, 2048.0, Dimension::Overworld);
    }

    let mut chunk_calc_schedule = Schedule::default();
    chunk_calc_schedule
        .add_systems((emit_chunk_calc_for(moving_player), chunk_calculator::handle).chain());
    chunk_calc_schedule.run(&mut world);

    let mut unload_schedule = Schedule::default();
    unload_schedule.add_systems(
        (
            entity_unloader::handle,
            save_mob_bundles,
            chunk_unloader::handle,
            handle_despawn_mob,
        )
            .chain(),
    );
    unload_schedule.run(&mut world);

    let mut fox_query = world.query::<(&Identity, Has<Fox>)>();
    let live_foxes: Vec<_> = fox_query
        .iter(&world)
        .filter(|(_, is_fox)| *is_fox)
        .map(|(identity, _)| identity.uuid)
        .collect();

    assert_eq!(
        live_foxes,
        vec![fox_uuid],
        "fox should stay live while another player still keeps the chunk visible"
    );
    assert!(
        state
            .0
            .world
            .is_chunk_cached(chunk.full(Dimension::Overworld)),
        "chunk should stay cached while a second player still has it loaded"
    );
}
