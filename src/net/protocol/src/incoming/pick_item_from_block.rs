use temper_codec::net_types::network_position::NetworkPosition;
use temper_macros::{NetDecode, packet};

/// Client-to-Server packet to request a "pick block" action.
#[derive(NetDecode, Debug)]
#[packet(packet_id = "pick_item_from_block", state = "play")]
pub struct PickItemFromBlock {
    /// The location of the block the player is looking at.
    pub location: NetworkPosition,
    /// True if the client wants the block's NBT data (creative only)
    pub include_data: bool,
}
