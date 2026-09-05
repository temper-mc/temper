use bevy_ecs::prelude::{Entity, Query};
use temper_command_infra::args::EntitiesArg;
use temper_command_infra::{CommandHandler, CommandResult, CommandSource};
use temper_components::entity_identity::Identity;
use temper_components::player::player_marker::PlayerMarker;
use temper_core::mq;
use temper_macros::Command;
use temper_permissions::Access::Allow;
use temper_permissions::Permissions;
use temper_permissions::player::PlayerPermission;
use temper_text::{Color, NamedColor, TextComponent, TextContent};

#[derive(Debug, Command)]
#[command(name = "op", permission = Permissions::Op)]
struct OpCommand {
    target: EntitiesArg,
}

impl CommandHandler for OpCommand {
    type SystemParam<'w, 's> = (
        Query<'w, 's, (Entity, &'static Identity, Option<&'static PlayerMarker>)>,
        Query<'w, 's, &'static mut PlayerPermission>,
    );

    fn handle(
        self,
        source: CommandSource,
        params: &mut Self::SystemParam<'_, '_>,
    ) -> CommandResult {
        let (entities, permissions) = params;

        for entity in self.target.resolve(entities.iter()) {
            if let Ok(mut player_permission) = permissions.get_mut(entity) {
                if player_permission.permissions.get(&Permissions::ALL) == Some(&Allow) {
                    source.send_message(TextComponent {
                        content: TextContent::Text {
                            text: "Nothing changed. The player is already an operator".to_string(),
                        },
                        color: Some(Color::Named(NamedColor::Red)),
                        ..Default::default()
                    });
                    continue;
                }

                player_permission.set_permission(Permissions::ALL, Allow);

                let Ok(Some(player_name)) = entities.get(entity).map(|e| &e.1.name) else {
                    return Err("player entity does not have a name".into());
                };

                source.send_message(TextComponent::from(format!(
                    "Made {} a server operator",
                    player_name
                )));

                mq::queue(
                    TextComponent::from("You have been opped".to_string()),
                    false,
                    entity,
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::message::MessageRegistry;
    use bevy_ecs::prelude::{Messages, Schedule};
    use bevy_ecs::system::SystemState;
    use std::sync::Arc;
    use temper_command_infra::{
        CommandDispatched, CommandHandler, CommandReader, CommandSource, CommandSpec,
        dispatch_command,
    };
    use temper_components::entity_identity::Identity;
    use temper_components::player::player_marker::PlayerMarker;
    use temper_permissions::Access::Allow;
    use temper_permissions::Permissions::{ALL, Op};
    use temper_permissions::player::PlayerPermission;

    use super::OpCommand;

    #[test]
    fn op_parses_target_arg() {
        let command = OpCommand::parse("Steve").unwrap();

        assert_eq!(&*command.target, "Steve");
    }

    #[test]
    fn op_grants_all_permission_to_resolved_target() {
        let mut world = bevy_ecs::prelude::World::new();
        let target = world
            .spawn((identity("Steve"), PlayerMarker, PlayerPermission::new()))
            .id();
        let command = OpCommand::parse("Steve").unwrap();

        let mut params =
            SystemState::<<OpCommand as CommandHandler>::SystemParam<'_, '_>>::new(&mut world);
        let mut system_params = params.get_mut(&mut world).unwrap();
        command
            .handle(CommandSource::Server, &mut system_params)
            .unwrap();
        params.apply(&mut world);

        assert!(
            world
                .get::<PlayerPermission>(target)
                .unwrap()
                .can(temper_permissions::Permissions::Kill)
        );
    }

    #[test]
    fn op_requires_op_permission_during_permission_aware_parse() {
        let mut reader = CommandReader::new("Steve");
        let err =
            OpCommand::parse_reader_with_permissions(&mut reader, &|permission| permission != Op)
                .unwrap_err();

        assert_eq!(err.expected, "permission");
    }

    #[test]
    fn op_dispatch_can_read_and_write_player_permissions() {
        let mut world = bevy_ecs::prelude::World::new();
        MessageRegistry::register_message::<CommandDispatched>(&mut world);
        let sender = world
            .spawn((
                identity("Sender"),
                PlayerMarker,
                player_permission(Op, Allow),
            ))
            .id();
        let target = world
            .spawn((identity("Steve"), PlayerMarker, PlayerPermission::new()))
            .id();
        world
            .resource_mut::<Messages<CommandDispatched>>()
            .write(CommandDispatched {
                input: Arc::from("op Steve"),
                source: CommandSource::Player(sender),
            });

        let mut schedule = Schedule::default();
        schedule.add_systems(dispatch_command::<OpCommand>);
        schedule.run(&mut world);

        assert!(world.get::<PlayerPermission>(target).unwrap().can(ALL));
    }

    fn identity(name: &str) -> Identity {
        Identity {
            uuid: uuid::Uuid::new_v4(),
            name: Some(name.to_string()),
        }
    }

    fn player_permission(
        permission: temper_permissions::Permissions,
        access: temper_permissions::Access,
    ) -> PlayerPermission {
        let mut player_permission = PlayerPermission::new();
        player_permission.set_permission(permission, access);
        player_permission
    }
}
