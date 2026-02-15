use std::fmt::Display;
use temper_macros::setup_packet_handling;

use bevy_ecs::prelude::*;
use std::sync::Arc;

pub mod errors;
pub mod incoming;
pub mod outgoing;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;

setup_packet_handling!("\\src\\incoming");

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum ConnState {
    Handshake,
    Login,
    Status,
    Configuration,
    Play,
}

impl Display for ConnState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnState::Handshake => write!(f, "Handshake"),
            ConnState::Login => write!(f, "Login"),
            ConnState::Status => write!(f, "Status"),
            ConnState::Configuration => write!(f, "Configuration"),
            ConnState::Play => write!(f, "Play"),
        }
    }
}
