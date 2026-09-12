use temper_codec::net_types::network_position::NetworkPosition;
use temper_macros::{NetDecode, packet};

#[derive(NetDecode, Debug)]
#[packet(packet_id = "sign_update", state = "play")]
pub struct SignUpdate {
    pub position: NetworkPosition,
    pub is_front_text: bool,
    pub line_1: String,
    pub line_2: String,
    pub line_3: String,
    pub line_4: String,
}
