use bevy_ecs::prelude::*;
use temper_commands::{arg::primitive::string::GreedyString, Sender};
use temper_components::player::player_identity::PlayerIdentity;
use temper_macros::command;
use temper_text::{TextComponent, TextComponentBuilder};

#[command("echo")]
fn test_command(
    #[arg] message: GreedyString,
    #[sender] sender: Sender,
    query: Query<&PlayerIdentity>,
) {
    let username = match sender {
        Sender::Server => "Server".to_string(),
        Sender::Player(entity) => query
            .get(entity)
            .expect("sender does not exist")
            .username
            .clone(),
    };

    sender.send_message(
        TextComponentBuilder::new(format!("{} said: ", username))
            .extra(TextComponent::from(message.clone()))
            .build(),
        false,
    );
}
