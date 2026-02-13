use bevy_ecs::prelude::*;
use temper_commands::Sender;
use temper_components::player::abilities::PlayerAbilities;
use temper_components::player::player_identity::PlayerIdentity;
use temper_macros::command;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::player_abilities::PlayerAbilities as OutgoingPlayerAbilities;
use temper_text::TextComponentBuilder;
use tracing::{error, info};

/// Toggles the sender's flying abilities.
#[command("fly")]
fn fly_command(
    #[sender] sender: Sender,
    mut player_query: Query<(Entity, &PlayerIdentity, &mut PlayerAbilities, &StreamWriter)>,
) {
    // 1. Ensure the sender is a player
    let player_entity = match sender {
        Sender::Server => {
            sender.send_message("Error: The server can't fly.".into(), false);
            return;
        }
        Sender::Player(entity) => entity,
    };

    // 2. Get all components for that player in one go
    let (_entity, identity, mut abilities, writer) = match player_query.get_mut(player_entity) {
        Ok(components) => components,
        Err(e) => {
            error!(
                "Failed to get components for player {:?}: {:?}",
                player_entity, e
            );
            sender.send_message(
                "Error: Could not find your player components.".into(),
                false,
            );
            return;
        }
    };

    // 3. Toggle the abilities
    abilities.may_fly = !abilities.may_fly;
    // Also toggle flying state so they don't fall when disabling
    if !abilities.may_fly {
        abilities.flying = false;
    }

    let status = if abilities.may_fly {
        "enabled"
    } else {
        "disabled"
    };

    // 4. Create and send the sync packet (we already have the `writer`)
    let sync_packet = OutgoingPlayerAbilities::from_abilities(&abilities);

    if let Err(e) = writer.send_packet_ref(&sync_packet) {
        error!(
            "Failed to send abilities sync packet to {}: {:?}",
            identity.username, e
        );
    }

    // 5. Send a confirmation message to the player
    sender.send_message(
        TextComponentBuilder::new(format!("Flying {}", status)).build(),
        false,
    );

    // 6. Log the action
    info!("Toggled flying for {}: {}", identity.username, status);
}
