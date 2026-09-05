use background::entity_tracking::refresh_visible_entities;
use bevy_ecs::prelude::{Schedule, World};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use temper_components::entity_identity::Identity;
use temper_components::game_id::GameID;
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_components::player::entity_tracker::EntityTracker;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::position::Position;
use temper_encryption::write::EncryptedWriter;
use temper_net_runtime::connection::StreamWriter;
use temper_state::create_test_state;
use tokio::net::{TcpListener, TcpStream};

async fn test_stream_writer() -> (StreamWriter, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener should bind");
    let addr = listener
        .local_addr()
        .expect("test listener should have a local address");

    let client_connect = TcpStream::connect(addr);
    let accept_connection = async {
        listener
            .accept()
            .await
            .map(|(stream, _)| stream)
            .expect("test listener should accept a client")
    };
    let (client_stream, server_stream) = tokio::join!(client_connect, accept_connection);
    let client_stream = client_stream.expect("test client should connect");

    let (_reader, writer) = server_stream.into_split();
    let (state, _temp_dir) = create_test_state();

    (
        StreamWriter::new(
            EncryptedWriter::from(writer),
            Arc::new(AtomicBool::new(true)),
            state.0.clone(),
            Arc::new(Mutex::new(None)),
        )
        .await,
        client_stream,
    )
}

#[tokio::test]
async fn visible_mobs_without_stream_writers_stay_tracked() {
    let (player_writer, _client_stream) = test_stream_writer().await;

    let mut world = World::new();
    let mob_position = Position::new(8.0, 64.0, 8.0);
    let mob_entity = world
        .spawn((Identity::new(None), GameID::new(), mob_position))
        .id();

    let mut chunk_receiver = ChunkReceiver::default();
    chunk_receiver.loaded.insert(mob_position.chunk());

    let mut tracker = EntityTracker::default();
    tracker.tracking.insert(mob_entity);

    world.spawn((
        Identity::new(Some("player".to_string())),
        Position::new(0.0, 64.0, 0.0),
        PlayerMarker,
        chunk_receiver,
        tracker,
        player_writer,
    ));

    let mut schedule = Schedule::default();
    schedule.add_systems(refresh_visible_entities);
    schedule.run(&mut world);

    let tracker = world
        .query::<&EntityTracker>()
        .single(&world)
        .expect("player tracker should exist");

    assert!(
        tracker.tracking.contains(&mob_entity),
        "visible mobs should remain tracked even though they have no StreamWriter"
    );
    assert!(
        tracker.to_untrack.pop().is_none(),
        "visible mobs should not be queued for removal"
    );
}
