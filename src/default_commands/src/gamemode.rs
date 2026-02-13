use bevy_ecs::prelude::*;
use temper_commands::Sender;
use temper_components::player::gamemode::GameMode;
use temper_macros::command;
use temper_messages::PlayerGameModeChanged;

/// Sets the sender's gamemode.
#[command("gamemode")]
#[allow(unused_mut)] // For the `player_query`
fn gamemode_command(
    #[sender] sender: Sender,
    #[arg] new_gamemode: GameMode,
    mut gamemode_events: MessageWriter<PlayerGameModeChanged>,
) {
    // 1. Ensure the sender is a player
    let player_entity = match sender {
        Sender::Server => {
            sender.send_message("Error: The server can't change gamemode.".into(), false);
            return;
        }
        Sender::Player(entity) => entity,
    };

    // 2. Fire the event
    gamemode_events.write(PlayerGameModeChanged {
        player: player_entity,
        new_mode: new_gamemode,
    });
}
