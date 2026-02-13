use bevy_ecs::prelude::*;
use ionic_components::player::player_identity::PlayerIdentity;
use ionic_core::mq;
use ionic_protocol::ChatMessagePacketReceiver;
use ionic_text::TextComponent;

pub fn handle(receiver: Res<ChatMessagePacketReceiver>, query: Query<&PlayerIdentity>) {
    for (message, sender) in receiver.0.try_iter() {
        let Ok(identity) = query.get(sender) else {
            continue;
        };

        let message = format!("<{}> {}", identity.username, message.message);
        mq::broadcast(TextComponent::from(message), false);
    }
}
