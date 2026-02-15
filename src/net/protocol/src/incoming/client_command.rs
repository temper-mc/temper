//! Client Command packet.
//!
//! Sent by the client to perform various actions:
//! - Action 0: Request respawn after death

#[allow(unused_imports)]
use temper_codec::net_types::var_int::VarInt;
use temper_macros::{NetDecode, packet};

/// Client command actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, NetDecode)]
#[net(type_cast = "VarInt", type_cast_handler = "value.0 as u8")]
#[repr(u8)]
pub enum ClientCommandAction {
    /// Request to respawn after death
    PerformRespawn = 0,
    /// Request game statistics (not implemented)
    RequestStats = 1,
}

/// Sent by the client to request respawn or stats.
#[derive(NetDecode, Debug)]
#[packet(packet_id = "client_command", state = "play")]
pub struct ClientCommand {
    /// The action to perform
    pub action: ClientCommandAction,
}

impl ClientCommand {
    /// Check if this is a respawn request.
    pub fn is_respawn_request(&self) -> bool {
        self.action == ClientCommandAction::PerformRespawn
    }
}
