use temper_codec::net_types::length_prefixed_vec::LengthPrefixedVec;
use temper_macros::{NetDecode, packet};

#[derive(NetDecode)]
#[packet(packet_id = "key", state = "login")]
pub struct EncryptionResponse {
    pub shared_secret: LengthPrefixedVec<u8>,
    pub verify_token: LengthPrefixedVec<u8>,
}
