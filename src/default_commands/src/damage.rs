use bevy_ecs::prelude::{Entity, MessageWriter, Query};
use temper_command_infra::args::{EntitiesArg, IntegerArg};
use temper_command_infra::{CommandHandler, CommandResult, CommandSource};
use temper_components::entity_identity::Identity;
use temper_components::player::player_marker::PlayerMarker;
use temper_macros::Command;
use temper_messages::damage::{DamageEvent, DamageSource};
use temper_permissions::Permissions;

#[derive(Command)]
#[command(name = "damage", permission = Permissions::Kill)]
struct DamageCommand {
    target: EntitiesArg,
    amount: IntegerArg<1, 32767>,
}

impl CommandHandler for DamageCommand {
    type SystemParam<'w, 's> = (
        Query<'w, 's, (Entity, &'static Identity, Option<&'static PlayerMarker>)>,
        MessageWriter<'w, DamageEvent>,
    );

    fn handle(
        self,
        source: CommandSource,
        params: &mut Self::SystemParam<'_, '_>,
    ) -> CommandResult {
        let (entities, damage_events) = params;
        let targets = self.target.resolve(entities.iter());

        if targets.is_empty() {
            return Err("No entities matched the target.".into());
        }

        for target in &targets {
            damage_events.write(DamageEvent {
                target: *target,
                source: DamageSource::DivineSmiting { silent: false },
                damage: *self.amount as u16,
                knockback: None,
                knockback_source: None,
            });
        }

        source.send_message(
            format!("Damaged {} entities for {}.", targets.len(), *self.amount).into(),
        );

        Ok(())
    }
}
