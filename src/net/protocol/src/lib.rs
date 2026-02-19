use std::fmt::Display;

use bevy_ecs::prelude::*;

pub mod errors;
pub mod incoming;
pub mod outgoing;

include!(concat!(env!("OUT_DIR"), "/packet_handling.rs"));

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
