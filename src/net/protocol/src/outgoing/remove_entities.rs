use temper_codec::net_types::length_prefixed_vec::LengthPrefixedVec;
use temper_codec::net_types::var_int::VarInt;
use temper_components::game_id::GameID;
use temper_macros::{NetEncode, packet};

#[derive(NetEncode)]
#[packet(packet_id = "remove_entities", state = "play")]
pub struct RemoveEntitiesPacket {
    pub entity_ids: LengthPrefixedVec<VarInt>,
}

impl RemoveEntitiesPacket {
    pub fn from_entities<T>(entity_ids: T) -> Self
    where
        T: IntoIterator<Item = GameID>,
    {
        let entity_ids: Vec<VarInt> = entity_ids.into_iter().map(|entity| entity.get()).collect();
        Self {
            entity_ids: LengthPrefixedVec::new(entity_ids),
        }
    }
}
