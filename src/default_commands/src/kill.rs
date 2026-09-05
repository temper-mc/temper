use bevy_ecs::prelude::{Entity, MessageWriter, Query};
use temper_command_infra::CommandSource::Player;
use temper_command_infra::args::EntityArg;
use temper_command_infra::{CommandHandler, CommandResult, CommandSource};
use temper_components::entity_identity::Identity;
use temper_components::player::player_marker::PlayerMarker;
use temper_macros::Command;
use temper_messages::damage::DamageSource::DivineSmiting;
use temper_messages::kill_entity::KillEntity;
use temper_permissions::Permissions;

#[derive(Command)]
#[command(name ="kill", permission = Permissions::Kill)]
enum KillCommand {
    SelfTarget,
    OtherTarget { target: EntityArg },
}

impl CommandHandler for KillCommand {
    type SystemParam<'w, 's> = (
        Query<'w, 's, (Entity, &'static Identity, Option<&'static PlayerMarker>)>,
        MessageWriter<'w, KillEntity>,
    );

    fn handle(
        self,
        source: CommandSource,
        params: &mut Self::SystemParam<'_, '_>,
    ) -> CommandResult {
        let &mut (query, ref mut writer) = params;

        let selected_entities = match self {
            KillCommand::SelfTarget => {
                if let Player(entity) = source {
                    vec![entity]
                } else {
                    return Err("The server cannot target itself with this command.".into());
                }
            }
            KillCommand::OtherTarget { target } => target.resolve(query.iter()),
        };

        selected_entities.iter().for_each(|e| {
            writer.write(KillEntity {
                entity: *e,
                message: Some("Killed by command.".into()),
                source: DivineSmiting { silent: true },
            });
        });

        source.send_message(
            format!(
                "Killed {} entities (excluding players).",
                selected_entities.len()
            )
            .into(),
        );

        Ok(())
    }
}
