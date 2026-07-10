use bevy_ecs::prelude::{Entity, Query, Res, With};
use temper_command_infra::{CommandHandler, CommandResult, CommandSource};
use temper_components::entity_identity::Identity;
use temper_components::player::player_marker::PlayerMarker;
use temper_macros::Command;
use temper_state::GlobalStateResource;

#[derive(Command)]
#[command(name = "list")]
enum ListCommand {
    Normal,
    #[literal("uuid")]
    Uuid,
}

impl CommandHandler for ListCommand {
    type SystemParam<'w, 's> = (
        Res<'w, GlobalStateResource>,
        Query<'w, 's, (Entity, &'static Identity), With<PlayerMarker>>,
    );

    fn handle(
        self,
        source: CommandSource,
        params: &mut Self::SystemParam<'_, '_>,
    ) -> CommandResult {
        let &mut (ref state, query) = params;

        let player_list = query
            .into_iter()
            .map(|(_, identity)| {
                let Some(ref player_name) = identity.name else {
                    return Err("player entity does not have a name");
                };

                Ok(match self {
                    ListCommand::Normal => player_name.clone(),
                    ListCommand::Uuid => format!("{player_name} ({})", identity.uuid),
                })
            })
            .collect::<Result<Vec<String>, _>>()?;

        source.send_message(
            format!(
                "There are {} of a max of {} players online: {}",
                player_list.len(),
                state.0.config.max_players,
                player_list.join(", "),
            )
            .into(),
        );

        Ok(())
    }
}
