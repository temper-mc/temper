use bevy_ecs::prelude::{MessageReader, Query};
use temper_components::entity_identity::Identity;
use temper_messages::damage::DamageSource::*;
use temper_messages::kill_entity::KillEntity;

pub fn send_death_message(mut deaths: MessageReader<KillEntity>, query: Query<&Identity>) {
    for death in deaths.read() {
        if matches!(death.source, DivineSmiting { silent: true }) {
            continue;
        }

        let killed_name = query
            .get(death.entity)
            .map(|identity| identity.name.clone().unwrap_or_else(|| "Unknown".into()))
            .unwrap_or_else(|_| "Unknown".into());

        temper_core::mq::broadcast(
            death.source.to_death_message(&killed_name, None).into(),
            false,
        );
    }
}
