use temper_macros::{NetEncode, packet};

#[derive(NetEncode, Copy, Clone)]
#[packet(packet_id = "entity_event", state = "play")]
pub struct EntityStatus {
    /// The ID of the entity
    pub entity_id: i32,
    /// The status code to send
    pub status: u8,
}
