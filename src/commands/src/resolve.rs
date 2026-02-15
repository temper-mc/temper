use crate::{Command, CommandContext, CommandInput, Sender, infrastructure};
use bevy_ecs::error;
use std::sync::Arc;
use temper_state::GlobalState;
use temper_text::{NamedColor, TextComponent, TextComponentBuilder};

pub fn resolve(
    input: String,
    sender: Sender,
    state: GlobalState,
) -> error::Result<(Arc<Command>, CommandContext), Box<TextComponent>> {
    let command = infrastructure::find_command(&input);
    if command.is_none() {
        return Err(Box::new(
            TextComponentBuilder::new("Unknown command")
                .color(NamedColor::Red)
                .build(),
        ));
    }

    let command = command.unwrap();
    let input = input
        .strip_prefix(command.name)
        .unwrap_or(&input)
        .trim_start();
    let input = CommandInput::of(input.to_string());
    let ctx = CommandContext {
        input: input.clone(),
        command: command.clone(),
        sender,
        state,
    };

    Ok((command, ctx))
}
