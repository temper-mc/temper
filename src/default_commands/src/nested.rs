use bevy_ecs::prelude::*;
use temper_commands::Sender;
use temper_components::player::player_identity::PlayerIdentity;
use temper_macros::command;
use temper_text::TextComponent;

#[command("nested")]
fn nested_command(#[sender] sender: Sender, query: Query<&PlayerIdentity>) {
    let username = match sender {
        Sender::Server => "Server".to_string(),
        Sender::Player(entity) => query
            .get(entity)
            .expect("sender does not exist")
            .username
            .clone(),
    };

    sender.send_message(
        TextComponent::from(format!("{} executed /nested", username)),
        false,
    );
}

#[command("nested nested")]
fn nested_nested_command(#[sender] sender: Sender, query: Query<&PlayerIdentity>) {
    let username = match sender {
        Sender::Server => "Server".to_string(),
        Sender::Player(entity) => query
            .get(entity)
            .expect("sender does not exist")
            .username
            .clone(),
    };

    sender.send_message(
        TextComponent::from(format!("{} executed /nested nested", username)),
        false,
    );
}
