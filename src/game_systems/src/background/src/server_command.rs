use bevy_ecs::change_detection::Res;
use bevy_ecs::message::MessageWriter;
use temper_commands::Sender;
use temper_commands::messages::{CommandDispatched, ResolvedCommandDispatched};
use temper_commands::resolve::resolve;
use temper_resources::server_command_rx::ServerCommandReceiver;
use temper_state::GlobalStateResource;
use tracing::error;

pub fn handle(
    receiver: Res<ServerCommandReceiver>,
    mut dispatch_msgs: MessageWriter<CommandDispatched>,
    mut resolved_dispatch_msgs: MessageWriter<ResolvedCommandDispatched>,
    state: Res<GlobalStateResource>,
) {
    for command in receiver.0.try_iter() {
        let sender = Sender::Server;
        dispatch_msgs.write(CommandDispatched {
            command: command.clone(),
            sender,
        });

        let resolved = resolve(command, sender, state.0.clone());
        match resolved {
            Err(err) => {
                error!("Error resolving server command: {}", err.to_plain_text());
            }

            Ok((command, ctx)) => {
                resolved_dispatch_msgs.write(ResolvedCommandDispatched {
                    command,
                    ctx,
                    sender,
                });
            }
        }
    }
}
