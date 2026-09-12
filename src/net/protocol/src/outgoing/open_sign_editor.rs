use temper_codec::net_types::network_position::NetworkPosition;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "open_sign_editor", state = "play")]
pub struct OpenSignEditor {
    pub location: NetworkPosition,
    pub is_front_text: bool,
}
