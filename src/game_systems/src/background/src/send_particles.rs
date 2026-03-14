use bevy_ecs::prelude::{MessageReader, Query};
use temper_components::player::position::Position;
use temper_messages::particle::SendParticle;
use temper_net_runtime::connection::StreamWriter;
use temper_protocol::outgoing::particle::Particle;
use tracing::warn;

pub fn handle(mut reader: MessageReader<SendParticle>, writers: Query<(&Position, &StreamWriter)>) {
    for msg in reader.read() {
        let packet = Particle {
            long_distance: false,
            always_visible: false,
            x: msg.position.x as f64,
            y: msg.position.y as f64,
            z: msg.position.z as f64,
            offset_x: msg.offset.x,
            offset_y: msg.offset.y,
            offset_z: msg.offset.z,
            max_speed: msg.speed,
            count: msg.count,
            particle_type: msg.particle_type.clone(),
        };

        for (pos, writer) in writers.iter() {
            let distance_sq = pos.as_vec3a().distance_squared(msg.position);
            if distance_sq <= 256.0 * 256.0
                && let Err(e) = writer.send_packet_ref(&packet)
            {
                warn!("Failed to send particle packet: {:?}", e);
            }
        }
    }
}
