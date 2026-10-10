use bevy_ecs::message::MessageRegistry;
use bevy_ecs::prelude::*;
use bevy_math::Vec3A;
use physics::{collisions, ground_state, velocity};
use temper_components::bounds::CollisionBounds;
use temper_components::entity_identity::Identity;
use temper_components::player::grounded::OnGround;
use temper_components::player::old_position::OldPosition;
use temper_components::player::player_bundle::PlayerBundle;
use temper_components::player::position::Position;
use temper_components::player::velocity::Velocity;
use temper_core::block_state_id::BlockStateId;
use temper_core::dimension::Dimension;
use temper_core::pos::{ChunkBlockPos, ChunkPos};
use temper_entities::PhysicalRegistry;
use temper_entities::bundles::PigBundle;
use temper_entities::markers::HasCollisions;
use temper_entities::markers::entity_types::Pig;
use temper_macros::block;
use temper_messages::damage::DamageEvent;
use temper_messages::entity_update::SendEntityUpdate;
use temper_messages::kill_entity::KillEntity;
use temper_state::create_test_state;

#[test]
fn falling_entity_lands_when_velocity_step_crosses_floor() {
    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(0, 64, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    let mut bundle = PigBundle::new(Position::new(0.5, 65.2, 0.5));
    bundle.velocity = Velocity::new(0.0, -2.4, 0.0);

    let entity = world.spawn((bundle, Pig, HasCollisions)).id();

    let mut schedule = Schedule::default();
    schedule.add_systems((ground_state::snapshot, velocity::handle, collisions::handle).chain());
    schedule.run(&mut world);

    let pos = world.get::<Position>(entity).unwrap();
    let vel = world.get::<Velocity>(entity).unwrap();
    let grounded = world.get::<OnGround>(entity).unwrap();

    assert_eq!(pos.coords.y, 65.0);
    assert_eq!(vel.vec.y, 0.0);
    assert!(grounded.currently_grounded);
}

#[test]
fn walking_entity_stops_flush_against_a_wall_on_a_partial_width_axis() {
    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(1, 64, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    // Pig is 0.9 blocks wide (half-width 0.45), which doesn't divide evenly into whole
    // blocks - this is what actually exercises the snapping math on a non-full-width axis.
    let mut bundle = PigBundle::new(Position::new(0.0, 64.0, 0.5));
    bundle.velocity = Velocity::new(2.0, 0.0, 0.0);

    let entity = world.spawn((bundle, Pig, HasCollisions)).id();

    let mut schedule = Schedule::default();
    schedule.add_systems((ground_state::snapshot, velocity::handle, collisions::handle).chain());
    schedule.run(&mut world);

    let pos = world.get::<Position>(entity).unwrap();
    let vel = world.get::<Velocity>(entity).unwrap();

    // Wall's min face is at x=1.0, half-width is 0.45, so the pig should stop with its
    // hitbox flush against the wall rather than snapping to a whole-block boundary.
    assert!(
        (pos.coords.x - 0.55).abs() < 1e-5,
        "expected x flush against wall at 0.55, got {}",
        pos.coords.x
    );
    assert_eq!(vel.vec.x, 0.0);
}

#[test]
fn diagonal_movement_slides_along_a_wall_instead_of_stopping_dead() {
    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(1, 64, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    let mut bundle = PigBundle::new(Position::new(0.0, 64.0, 0.5));
    bundle.velocity = Velocity::new(2.0, 0.0, 1.5);

    let entity = world.spawn((bundle, Pig, HasCollisions)).id();

    let mut schedule = Schedule::default();
    schedule.add_systems((ground_state::snapshot, velocity::handle, collisions::handle).chain());
    schedule.run(&mut world);

    let pos = world.get::<Position>(entity).unwrap();
    let vel = world.get::<Velocity>(entity).unwrap();

    assert!(
        (pos.coords.x - 0.55).abs() < 1e-5,
        "expected x flush against wall at 0.55, got {}",
        pos.coords.x
    );
    assert!(
        (pos.coords.z - 2.0).abs() < 1e-5,
        "expected z to travel unobstructed to 2.0, got {}",
        pos.coords.z
    );
    assert_eq!(vel.vec.x, 0.0);
    assert_eq!(vel.vec.z, 1.5);
}

#[test]
fn player_landing_on_ground_gets_marked_grounded_without_position_correction() {
    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(0, 64, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    // The client already reported landing a bit into the floor - we shouldn't correct a
    // player's Position (that's left to the client), but OnGround still needs to flip.
    let requested_pos = Position::new(0.5, 64.98, 0.5);
    let mut bundle = PlayerBundle {
        identity: Identity::new(Some("Steve".to_string())),
        position: requested_pos,
        collision_bounds: CollisionBounds::new(
            Vec3A::new(-0.3, 0.0, -0.3),
            Vec3A::new(0.3, 1.8, 0.3),
        ),
        ..Default::default()
    };
    bundle.on_ground = OnGround {
        currently_grounded: false,
        was_grounded: false,
    };

    let entity = world
        .spawn((
            bundle,
            HasCollisions,
            OldPosition::from(Position::new(0.5, 65.2, 0.5)),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems((ground_state::snapshot, collisions::handle).chain());
    schedule.run(&mut world);

    let pos = world.get::<Position>(entity).unwrap();
    let grounded = world.get::<OnGround>(entity).unwrap();

    // Position is left untouched - players are client-authoritative.
    assert_eq!(pos.coords, requested_pos.coords);
    assert!(grounded.currently_grounded);
}

#[test]
fn player_predicted_next_step_can_detect_an_upcoming_landing() {
    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(0, 64, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    let requested_pos = Position::new(0.5, 65.3, 0.5);
    let mut bundle = PlayerBundle {
        identity: Identity::new(Some("Steve".to_string())),
        position: requested_pos,
        collision_bounds: CollisionBounds::new(
            Vec3A::new(-0.3, 0.0, -0.3),
            Vec3A::new(0.3, 1.8, 0.3),
        ),
        ..Default::default()
    };
    bundle.on_ground = OnGround {
        currently_grounded: false,
        was_grounded: false,
    };

    let entity = world
        .spawn((
            bundle,
            HasCollisions,
            OldPosition::from(Position::new(0.5, 65.7, 0.5)),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems((ground_state::snapshot, collisions::handle).chain());
    schedule.run(&mut world);

    let grounded = world.get::<OnGround>(entity).unwrap();
    assert!(
        grounded.currently_grounded,
        "expected the predicted next player step to detect the upcoming landing"
    );
}

#[test]
fn player_landing_with_negligible_horizontal_jitter_still_detected() {
    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(0, 64, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    let requested_pos = Position::new(0.5000001, 64.98, 0.5);
    let mut bundle = PlayerBundle {
        identity: Identity::new(Some("Steve".to_string())),
        position: requested_pos,
        collision_bounds: CollisionBounds::new(
            Vec3A::new(-0.3, 0.0, -0.3),
            Vec3A::new(0.3, 1.8, 0.3),
        ),
        ..Default::default()
    };
    bundle.on_ground = OnGround {
        currently_grounded: false,
        was_grounded: false,
    };

    let entity = world
        .spawn((
            bundle,
            HasCollisions,
            OldPosition::from(Position::new(0.5, 65.2, 0.5)),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems((ground_state::snapshot, collisions::handle).chain());
    schedule.run(&mut world);

    let grounded = world.get::<OnGround>(entity).unwrap();
    assert!(
        grounded.currently_grounded,
        "expected landing to be detected despite tiny horizontal jitter"
    );
}

#[test]
fn player_walking_into_a_wall_gets_detected() {
    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(1, 65, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    let requested_pos = Position::new(1.0, 65.0, 0.5);
    let mut bundle = PlayerBundle {
        identity: Identity::new(Some("Steve".to_string())),
        position: requested_pos,
        collision_bounds: CollisionBounds::new(
            Vec3A::new(-0.3, 0.0, -0.3),
            Vec3A::new(0.3, 1.8, 0.3),
        ),
        ..Default::default()
    };
    bundle.on_ground = OnGround {
        currently_grounded: true,
        was_grounded: false,
    };

    let entity = world
        .spawn((
            bundle,
            HasCollisions,
            OldPosition::from(Position::new(0.5, 65.0, 0.5)),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems((ground_state::snapshot, collisions::handle).chain());
    schedule.run(&mut world);

    let grounded = world.get::<OnGround>(entity).unwrap();
    assert!(grounded.currently_grounded);
}

#[test]
fn player_landing_over_multiple_small_ticks_still_detected() {
    fn run_tick(world: &mut World, entity: Entity, new_pos: Position) {
        {
            let mut pos = world.get_mut::<Position>(entity).unwrap();
            let previous = *pos;
            *pos = new_pos;
            if let Some(mut old_pos) = world.get_mut::<OldPosition>(entity) {
                *old_pos = OldPosition::from(previous);
            } else {
                world.entity_mut(entity).insert(OldPosition::from(previous));
            }
        }
        let mut schedule = Schedule::default();
        schedule.add_systems((ground_state::snapshot, collisions::handle).chain());
        schedule.run(world);
    }

    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(0, 64, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    let mut bundle = PlayerBundle {
        identity: Identity::new(Some("Steve".to_string())),
        position: Position::new(0.5, 66.0, 0.5),
        collision_bounds: CollisionBounds::new(
            Vec3A::new(-0.3, 0.0, -0.3),
            Vec3A::new(0.3, 1.8, 0.3),
        ),
        ..Default::default()
    };
    bundle.on_ground = OnGround {
        currently_grounded: false,
        was_grounded: false,
    };

    let entity = world.spawn((bundle, HasCollisions)).id();

    run_tick(&mut world, entity, Position::new(0.5, 65.5, 0.5));
    run_tick(&mut world, entity, Position::new(0.5, 65.05, 0.5));
    run_tick(&mut world, entity, Position::new(0.5, 64.98, 0.5));

    let grounded = world.get::<OnGround>(entity).unwrap();
    assert!(
        grounded.currently_grounded,
        "expected landing to be detected across successive purely-vertical ticks"
    );
}

#[test]
fn player_landing_flush_on_a_whole_number_boundary_still_detected() {
    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(0, 71, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    let mut bundle = PlayerBundle {
        identity: Identity::new(Some("Steve".to_string())),
        position: Position::new(0.5, 72.0, 0.5),
        collision_bounds: CollisionBounds::new(
            Vec3A::new(-0.3, 0.0, -0.3),
            Vec3A::new(0.3, 1.8, 0.3),
        ),
        ..Default::default()
    };
    bundle.on_ground = OnGround {
        currently_grounded: false,
        was_grounded: false,
    };

    let entity = world
        .spawn((
            bundle,
            HasCollisions,
            OldPosition::from(Position::new(0.5, 72.1213, 0.5)),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems((ground_state::snapshot, collisions::handle).chain());
    schedule.run(&mut world);

    let grounded = world.get::<OnGround>(entity).unwrap();
    assert!(
        grounded.currently_grounded,
        "expected landing flush on a whole-number boundary to still be detected"
    );
}

#[test]
fn player_jumping_off_ground_does_not_get_falsely_marked_as_hitting_it() {
    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(0, 71, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    let mut bundle = PlayerBundle {
        identity: Identity::new(Some("Steve".to_string())),
        position: Position::new(0.5, 72.42, 0.5),
        collision_bounds: CollisionBounds::new(
            Vec3A::new(-0.3, 0.0, -0.3),
            Vec3A::new(0.3, 1.8, 0.3),
        ),
        ..Default::default()
    };
    bundle.on_ground = OnGround {
        currently_grounded: false,
        was_grounded: false,
    };

    let entity = world
        .spawn((
            bundle,
            HasCollisions,
            OldPosition::from(Position::new(0.5, 72.0, 0.5)),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems((ground_state::snapshot, collisions::handle).chain());
    schedule.run(&mut world);

    let grounded = world.get::<OnGround>(entity).unwrap();
    assert!(
        !grounded.currently_grounded,
        "jumping away from flush ground contact should not re-report a ground hit"
    );
}

#[test]
fn repeated_predicted_player_landing_only_marks_the_first_tick_as_new_ground_contact() {
    fn run_tick(world: &mut World, entity: Entity, new_pos: Position, reported_on_ground: bool) {
        {
            let mut pos = world.get_mut::<Position>(entity).unwrap();
            let previous = *pos;
            *pos = new_pos;
            if let Some(mut old_pos) = world.get_mut::<OldPosition>(entity) {
                *old_pos = OldPosition::from(previous);
            } else {
                world.entity_mut(entity).insert(OldPosition::from(previous));
            }
        }

        {
            let mut grounded = world.get_mut::<OnGround>(entity).unwrap();
            grounded.snapshot();
            grounded.currently_grounded = reported_on_ground;
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(collisions::handle);
        schedule.run(world);
    }

    let mut world = World::new();
    let (state, _temp_dir) = create_test_state();

    {
        let mut chunk = state
            .0
            .world
            .get_or_generate_mut(ChunkPos::new(0, 0), Dimension::Overworld)
            .expect("Failed to create test chunk");
        chunk.set_block(ChunkBlockPos::new(0, 64, 0), block!("stone"));
    }

    world.insert_resource(state);
    world.insert_resource(PhysicalRegistry::new());
    MessageRegistry::register_message::<SendEntityUpdate>(&mut world);
    MessageRegistry::register_message::<DamageEvent>(&mut world);
    MessageRegistry::register_message::<KillEntity>(&mut world);

    let mut bundle = PlayerBundle {
        identity: Identity::new(Some("Steve".to_string())),
        position: Position::new(0.5, 65.7, 0.5),
        collision_bounds: CollisionBounds::new(
            Vec3A::new(-0.3, 0.0, -0.3),
            Vec3A::new(0.3, 1.8, 0.3),
        ),
        ..Default::default()
    };
    bundle.on_ground = OnGround {
        currently_grounded: false,
        was_grounded: false,
    };

    let entity = world.spawn((bundle, HasCollisions)).id();

    run_tick(&mut world, entity, Position::new(0.5, 65.3, 0.5), false);

    let grounded = world.get::<OnGround>(entity).unwrap();
    assert!(grounded.currently_grounded);
    assert!(
        !grounded.was_grounded,
        "the first predictive landing should look like a new ground contact"
    );

    run_tick(&mut world, entity, Position::new(0.5, 64.9, 0.5), false);

    let grounded = world.get::<OnGround>(entity).unwrap();
    assert!(grounded.currently_grounded);
    assert!(
        grounded.was_grounded,
        "the second predictive landing tick should preserve that the entity was already grounded"
    );
}
