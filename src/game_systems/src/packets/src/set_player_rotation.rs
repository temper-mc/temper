use bevy_ecs::prelude::{MessageWriter, Query, Res};
use ionic_components::player::grounded::OnGround;
use ionic_components::player::rotation::Rotation;
use ionic_components::player::teleport_tracker::TeleportTracker;
use ionic_messages::packet_messages::Movement;
use ionic_protocol::SetPlayerRotationPacketReceiver;

pub fn handle(
    receiver: Res<SetPlayerRotationPacketReceiver>,
    mut movement_messages: MessageWriter<Movement>,
    mut query: Query<(&mut Rotation, &mut OnGround, &mut TeleportTracker)>,
) {
    for (event, eid) in receiver.0.try_iter() {
        if let Ok((mut rot, mut ground, tracker)) = query.get_mut(eid) {
            if tracker.waiting_for_confirm {
                // Ignore rotation updates while waiting for teleport confirmation
                continue;
            }
            let new_rot = Rotation::new(event.yaw, event.pitch);
            let on_ground = event.flags & 0x01 != 0;

            // Build movement message (rotation only, no position delta)
            let movement = Movement::new(eid).rotation(new_rot).on_ground(on_ground);

            // Update components
            *rot = new_rot;
            *ground = OnGround(on_ground);

            // Send movement message for broadcasting
            movement_messages.write(movement);
        }
    }
}
