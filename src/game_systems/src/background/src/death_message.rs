use bevy_ecs::prelude::{MessageReader, Query};
use rand::prelude::IndexedRandom;
use temper_components::entity_identity::Identity;
use temper_messages::damage::DamageSource::*;
use temper_messages::kill_entity::KillEntity;
use temper_text::TextComponent;

pub fn send_death_message(mut deaths: MessageReader<KillEntity>, query: Query<(&Identity)>) {
    let mut rng = rand::rng();
    
    for death in deaths.read() {
        if matches!(death.source, DivineSmiting { silent: true }) {
            continue;
        }
        
        let killed_name = query
            .get(death.entity)
            .map(|identity| identity.name.clone().unwrap_or_else(|| "Unknown".into()))
            .unwrap_or_else(|_| "Unknown".into());
        
        let message = match death.source {
            DivineSmiting { .. } => TextComponent::from(
                [
                    format!("{} was divinely smote", killed_name),
                    format!("{} was struck down by the gods", killed_name),
                ]
                .choose(&mut rng)
                .expect("Failed to choose a death message")
                .clone(),
            ),
            
            _ => TextComponent::from(format!("{} was killed", killed_name)),
        };
        
        temper_core::mq::broadcast(message, false);
    }
}
