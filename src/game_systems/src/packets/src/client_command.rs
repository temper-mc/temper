use bevy_ecs::prelude::{MessageWriter, Query, Res};
use temper_codec::net_types::var_int::VarInt;
use temper_components::player::chunk_receiver::ChunkReceiver;
use temper_components::player::gamemode::GameModeComponent;
use temper_messages::chunk_calc::ChunkCalc;
use temper_messages::teleport_entity::TeleportEntity;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::ClientCommandReceiver;
use temper_protocol::incoming::client_command::ClientCommandAction;
use temper_protocol::outgoing::game_event::GameEventPacket;
use temper_protocol::outgoing::respawn::RespawnPacket;
use temper_state::GlobalStateResource;
use tracing::error;

pub fn handle_client_command(
    events: Res<ClientCommandReceiver>,
    mut query: Query<(&StreamWriter, &GameModeComponent, &mut ChunkReceiver)>,
    mut chunk_calc_writer: MessageWriter<ChunkCalc>,
    mut pos_writer: MessageWriter<TeleportEntity>,
    state: Res<GlobalStateResource>,
) {
    for (message, sender) in events.0.try_iter() {
        match message.action {
            ClientCommandAction::PerformRespawn => {
                let (conn, gamemode, _chunk_recv) = query
                    .get_mut(sender)
                    .expect("No StreamWriter or GameModeComponent for sender");

                let packet = RespawnPacket {
                    dimension_type: VarInt::new(0),
                    dimension_name: "minecraft:overworld",
                    seed_hash: 0,
                    gamemode: gamemode.0,
                    previous_gamemode: -1,
                    is_debug: false,
                    is_flat: false,
                    data_kept: 0,
                    has_death_location: false,
                    death_dimension_name: None,
                    death_location: None,
                    portal_cooldown: Default::default(),
                    sea_level: Default::default(),
                };
                if let Err(err) = conn.send_packet(packet) {
                    error!("Failed to send respawn packet: {:?}", err);
                }
                let game_event = GameEventPacket {
                    event_id: 13,
                    value: 0.0,
                };
                if let Err(err) = conn.send_packet(game_event) {
                    error!("Failed to send game event packet: {:?}", err);
                }
                // *chunk_recv = ChunkReceiver::default();
                chunk_calc_writer.write(ChunkCalc(sender));
                pos_writer.write(TeleportEntity {
                    entity: sender,
                    position: state
                        .0
                        .spawn_positions
                        .pop()
                        .expect("No spawn position available")
                        .into(),
                    rotation: Default::default(),
                    velocity: Default::default(),
                });
            }
            ClientCommandAction::GameRules => {}
            ClientCommandAction::RequestStats => {}
        }
    }
}
