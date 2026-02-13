use bevy_ecs::prelude::*;
use ionic_commands::Sender;
use ionic_components::player::player_identity::PlayerIdentity;
use ionic_macros::command;
use ionic_text::TextComponent;

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
