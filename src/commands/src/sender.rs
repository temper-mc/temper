//! Command senders.

use bevy_ecs::prelude::*;
use temper_core::mq;
use temper_text::TextComponent;
use tracing::info;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
/// A possible command sender.
pub enum Sender {
    /// A player has sent a command.
    Player(Entity),

    /// The server console has sent a command.
    Server,
}

impl Sender {
    /// Sends the given `message` to this sender, and to the action bar
    /// if `actionbar` is true.
    pub fn send_message(&self, message: TextComponent, actionbar: bool) {
        match self {
            Sender::Player(entity) => mq::queue(message, actionbar, *entity),
            Sender::Server => {
                info!("{}", message.to_plain_text()); // TODO: serialize into ANSI?
            }
        }
    }
}
