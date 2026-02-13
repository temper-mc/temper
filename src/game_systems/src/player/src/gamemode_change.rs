use bevy_ecs::prelude::*;
use ionic_components::player::abilities::PlayerAbilities;
use ionic_components::player::gamemode::GameModeComponent;
use ionic_components::player::player_identity::PlayerIdentity;
use ionic_messages::PlayerGameModeChanged;
use ionic_net_runtime::connection::StreamWriter;
use ionic_protocol::outgoing::game_event::GameEventPacket;
use ionic_protocol::outgoing::player_abilities::PlayerAbilities as OutgoingAbilities;
use ionic_protocol::outgoing::system_message::SystemMessagePacket;
use ionic_text::{Color, NamedColor, TextComponent, TextComponentBuilder};
use tracing::{error, info};

/// Listens for `ChangeGameModeEvent` and applies all game logic.
pub fn handle(
    mut events: MessageReader<PlayerGameModeChanged>,
    mut player_query: Query<(
        &PlayerIdentity,
        &mut PlayerAbilities,
        &mut GameModeComponent,
        &StreamWriter,
    )>,
) {
    for event in events.read() {
        // 1. Get all the player's components
        let Ok((identity, mut abilities, mut gamemode, writer)) =
            player_query.get_mut(event.player)
        else {
            // Player might have disconnected in the same tick
            continue;
        };

        let new_mode = event.new_mode;

        // --- 2. Actual logic part ---

        // Update server-side components
        gamemode.0 = new_mode;
        new_mode.update_abilities(&mut abilities); // Use the helper you wrote

        // --- 3. Send sync packets to client ---

        // 3a. Game Event packet (changes the client's UI, e.g., hearts)
        let gamemode_packet = GameEventPacket::new(3, new_mode as u8 as f32);
        if let Err(e) = writer.send_packet_ref(&gamemode_packet) {
            error!(
                "Failed to send gamemode change packet to {}: {:?}",
                identity.username, e
            );
        }

        // 3b. Abilities packet (allows/disallows flight)
        let abilities_packet = OutgoingAbilities::from_abilities(&abilities);
        if let Err(e) = writer.send_packet_ref(&abilities_packet) {
            error!(
                "Failed to send abilities sync packet to {}: {:?}",
                identity.username, e
            );
        }

        // 4. Send confirmation chat message
        let mode_name = match new_mode {
            ionic_components::player::gamemode::GameMode::Survival => "Survival",
            ionic_components::player::gamemode::GameMode::Creative => "Creative",
            ionic_components::player::gamemode::GameMode::Adventure => "Adventure",
            ionic_components::player::gamemode::GameMode::Spectator => "Spectator",
        };

        let msg = TextComponentBuilder::new("Set gamemode to ")
            .extra(TextComponent::from(mode_name).color(Color::Named(NamedColor::Aqua)))
            .build();

        let chat_packet = SystemMessagePacket {
            message: msg.into(),
            overlay: false,
        };
        if let Err(e) = writer.send_packet_ref(&chat_packet) {
            error!(
                "Failed to send gamemode confirmation message to {}: {:?}",
                identity.username, e
            );
        }

        info!("Set gamemode for {} to {:?}", identity.username, new_mode);
    }
}
