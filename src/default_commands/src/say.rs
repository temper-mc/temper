use bevy_ecs::prelude::Query;
use temper_commands::arg::primitive::string::GreedyString;
use temper_commands::Sender;
use temper_components::player::player_identity::PlayerIdentity;
use temper_core::mq;
use temper_macros::command;
use temper_net_runtime::connection::StreamWriter;

#[command("say")]
fn say_command(
    #[sender] sender: Sender,
    #[arg] message: GreedyString,
    query: Query<(&StreamWriter, &PlayerIdentity)>,
) {
    let full_message = match sender {
        Sender::Server => format!("<Server> {}", message.as_str()),
        Sender::Player(entity) => {
            let player_identity = query.get(entity).expect("sender does not exist").1;
            format!("<{}> {}", player_identity.username, message.as_str())
        }
    };

    mq::broadcast(full_message.into(), false);
}
