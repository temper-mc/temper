use bevy_ecs::prelude::Query;
use bevy_ecs::prelude::{MessageWriter, Res};
use temper_components::player::grounded::OnGround;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_components::player::teleport_tracker::TeleportTracker;
use temper_messages::chunk_calc::ChunkCalc;
use temper_messages::packet_messages::Movement;
use temper_protocol::SetPlayerPositionAndRotationPacketReceiver;

pub fn handle(
    receiver: Res<SetPlayerPositionAndRotationPacketReceiver>,
    mut movement_messages: MessageWriter<Movement>,
    mut chunk_calc_messages: MessageWriter<ChunkCalc>,
    mut query: Query<(
        &mut Position,
        &mut Rotation,
        &mut OnGround,
        &mut TeleportTracker,
    )>,
) {
    for (event, eid) in receiver.0.try_iter() {
        if let Ok((mut pos, mut rot, mut ground, tracker)) = query.get_mut(eid) {
            if tracker.waiting_for_confirm {
                // Ignore position updates while waiting for teleport confirmation
                continue;
            }
            let new_pos = Position::new(event.x, event.feet_y, event.z);
            let new_rot = Rotation::new(event.yaw, event.pitch);
            let on_ground = event.flags & 0x01 != 0;

            // Check if chunk changed
            let old_chunk = (pos.x as i32 >> 4, pos.z as i32 >> 4);
            let new_chunk = (new_pos.x as i32 >> 4, new_pos.z as i32 >> 4);
            if old_chunk != new_chunk {
                chunk_calc_messages.write(ChunkCalc(eid));
            }

            // Build movement message with delta BEFORE updating component
            let movement = Movement::new(eid)
                .position_delta_from(&pos, &new_pos)
                .rotation(new_rot)
                .on_ground(on_ground);

            // Update components
            if pos.coords != new_pos.coords {
                *pos = new_pos;
            }
            if rot.yaw != new_rot.yaw || rot.pitch != new_rot.pitch {
                *rot = new_rot;
            }
            *ground = OnGround(on_ground);

            // Send movement message for broadcasting
            movement_messages.write(movement);
        }
    }
}
