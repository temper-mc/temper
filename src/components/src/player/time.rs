use bevy_ecs::component::Component;
use std::time::Instant;

#[derive(Component)]
pub struct LastSentTimeUpdate {
    timestamp: Instant,
    send_immediately: bool,
}

impl LastSentTimeUpdate {
    pub fn reset(&mut self) {
        self.timestamp = Instant::now();
        self.send_immediately = false;
    }

    pub fn send_next_tick(&mut self) {
        self.send_immediately = true;
    }

    pub fn should_resend(&self) -> bool {
        self.timestamp.elapsed().as_secs() >= 5 || self.send_immediately
    }
}

impl Default for LastSentTimeUpdate {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            send_immediately: false,
        }
    }
}
