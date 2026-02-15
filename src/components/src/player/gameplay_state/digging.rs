use bevy_ecs::prelude::Component;
use std::time::{Duration, Instant};
use temper_codec::net_types::network_position::NetworkPosition;

/// An "action component" added to a player when they start digging.
#[derive(Component, Debug, Clone)]
pub struct PlayerDigging {
    pub block_pos: NetworkPosition,
    pub start_time: Instant,
    pub break_time: Duration,
}
