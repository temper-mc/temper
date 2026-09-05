use bevy_ecs::prelude::{Commands, MessageWriter, Res};
use std::time::Instant;
use temper_components::bounds::CollisionBounds;
use temper_components::player::bossbar_sender::BossbarSender;
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_components::player::entity_tracker::EntityTracker;
use temper_components::player::grounded::OnGround;
use temper_components::player::keepalive::KeepAliveTracker;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::teleport_tracker::TeleportTracker;
use temper_components::player::{
    gamemode::GameModeComponent, offline_player_data::OfflinePlayerData,
    pending_events::PendingPlayerJoin, player_bundle::PlayerBundle, sneak::SneakState,
    swimming::SwimmingState,
};
use temper_inventories::hotbar::Hotbar;
use temper_messages::chunk_calc::ChunkCalc;
use temper_net_runtime::connection::DisconnectHandle;
use temper_resources::new_conn::NewConnectionRecv;
use temper_state::GlobalStateResource;
use tracing::{error, info};

pub fn accept_new_connections(
    mut cmd: Commands,
    new_connections: Res<NewConnectionRecv>,
    state: Res<GlobalStateResource>,
    mut chunk_update_writer: MessageWriter<ChunkCalc>,
) {
    if new_connections.0.is_empty() {
        return;
    }
    while let Ok(new_connection) = new_connections.0.try_recv() {
        let return_sender = new_connection.entity_return;

        // --- 1. Load all data from cache ---
        let offline_data_opt = match state
            .0
            .world
            .load_player_data(new_connection.player_identity.uuid)
        {
            Ok(data) => data,
            Err(err) => {
                error!(
                    "Error loading player data for {}: {:?}",
                    new_connection
                        .player_identity
                        .name
                        .as_ref()
                        .expect("No Player Name"),
                    err
                );
                None
            }
        };

        let player_data: OfflinePlayerData = offline_data_opt.expect(
            "No offline player data found for player, this should never happen as we create it on first join",
        );

        // --- 2. Build the PlayerBundle ---
        let player_bundle = PlayerBundle {
            identity: new_connection.player_identity.clone(),
            game_id: new_connection.game_id,
            abilities: player_data.abilities,
            player_properties: new_connection.player_properties,
            gamemode: GameModeComponent(player_data.gamemode),
            position: player_data.position.into(),
            rotation: player_data.rotation,
            on_ground: OnGround::default(),
            chunk_receiver: ChunkReceiver::default(),
            inventory: player_data.inventory,
            hotbar: Hotbar::default(),
            ender_chest: player_data.ender_chest,
            health: player_data.health,
            hunger: player_data.hunger,
            experience: player_data.experience,
            active_effects: player_data.active_effects,
            swimming: SwimmingState::default(),
            sneak: SneakState::default(),
            collision_bounds: CollisionBounds {
                x_offset_start: -0.3,
                x_offset_end: 0.3,
                y_offset_start: 0.0,
                y_offset_end: 1.8,
                z_offset_start: -0.3,
                z_offset_end: 0.3,
            },
            player_marker: PlayerMarker,
            entity_tracker: EntityTracker::default(),
            permissions: new_connection.permissions,
            bossbar_sender: BossbarSender::default(),
        };

        // --- 3. Spawn the PlayerBundle, then .insert() the network components ---
        let mut entity_commands = cmd.spawn(player_bundle);

        // Add network components and the pending join marker.
        // The marker triggers `emit_player_joined` to fire the actual event
        // after `apply_deferred` flushes the entity into existence.
        entity_commands.insert((
            new_connection.stream,
            new_connection.client_information_component,
            DisconnectHandle {
                sender: Some(new_connection.disconnect_handle),
            },
            KeepAliveTracker {
                last_sent_keep_alive_id: 0,
                last_received_keep_alive: Instant::now(),
                has_received_keep_alive: true,
                last_sent_keep_alive: Instant::now(),
            },
            PendingPlayerJoin(new_connection.player_identity.clone()),
            TeleportTracker {
                waiting_for_confirm: false,
            },
        ));

        let entity_id = entity_commands.id();

        // Add the new player to the global player list (used for server list player count)
        state.0.players.player_list.insert(
            entity_id,
            (
                new_connection.player_identity.uuid.as_u128(),
                new_connection
                    .player_identity
                    .name
                    .as_ref()
                    .expect("No Player Name")
                    .clone(),
            ),
        );

        chunk_update_writer.write(ChunkCalc(entity_id));

        info!(
            "Player {} connected ({:?})",
            new_connection
                .player_identity
                .name
                .as_ref()
                .expect("No Player Name"),
            new_connection.player_identity.uuid
        );

        if let Err(err) = return_sender.send(entity_id) {
            error!(
                "Failed to send entity ID back to the networking thread: {:?}",
                err
            );
        }
    }
}
