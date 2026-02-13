use bevy_ecs::prelude::*;
use temper_core::mq;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::system_message::SystemMessagePacket;
use temper_state::GlobalStateResource;
use tracing::{error, info};

fn send(
    writer: &StreamWriter,
    receiver: Entity,
    state: &GlobalStateResource,
    entry: mq::QueueEntry,
) {
    if !state.0.players.is_connected(receiver) {
        return;
    }

    if let Err(err) = writer.send_packet(SystemMessagePacket {
        message: entry.message.into(),
        overlay: entry.overlay,
    }) {
        error!("failed sending queued message to player: {err}");
    }
}

pub fn process(query: Query<(Entity, &StreamWriter)>, state: Res<GlobalStateResource>) {
    while !mq::QUEUE.is_empty() {
        let entry = mq::QUEUE.pop().unwrap();

        match entry.receiver {
            Some(receiver) => {
                let Ok((_, writer)) = query.get(receiver) else {
                    continue;
                };

                send(writer, receiver, &state, entry);
            }

            // None => broadcast to Everyone
            None => {
                for (receiver, writer) in query {
                    send(writer, receiver, &state, entry.clone());
                }
                info!("{}", entry.message.to_plain_text())
            }
        }
    }
}
