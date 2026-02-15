use temper_codec::net_types::length_prefixed_vec::LengthPrefixedVec;
use temper_macros::{NetDecode, packet};

#[derive(Debug, NetDecode)]
#[packet(packet_id = "select_known_packs", state = "configuration")]
pub struct ServerBoundKnownPacks {
    pub packs: LengthPrefixedVec<PackOwned>,
}

#[derive(Debug, NetDecode)]
#[expect(dead_code)]
pub struct PackOwned {
    namespace: String,
    id: String,
    version: String,
}
