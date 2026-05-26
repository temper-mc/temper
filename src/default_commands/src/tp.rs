use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{MessageWriter, Query};
use temper_commands::arg::entities::EntityArgument;
use temper_commands::arg::position::CommandPosition;
use temper_commands::Sender;
use temper_commands::Sender::Player;
use temper_components::entity_identity::Identity;
use temper_components::player::player_marker::PlayerMarker;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_macros::command;
use temper_messages::teleport_player::TeleportPlayer;

pub enum TeleportTarget {
    Position(CommandPosition),
    Entity(EntityArgument),
}

pub struct TpArgument {
    pub from: Option<EntityArgument>,
    pub to: TeleportTarget,
}

impl temper_commands::arg::CommandArgument for TpArgument {
    fn parse(ctx: &mut temper_commands::CommandContext) -> temper_commands::arg::ParserResult<Self> {
        // Read all remaining input into words
        let mut words = Vec::new();
        while ctx.input.has_remaining_input() {
            let word = ctx.input.read_string();
            if !word.is_empty() {
                words.push(word);
            }
        }

        if words.is_empty() {
            return Err(temper_commands::arg::utils::parser_error("missing arguments"));
        }

        fn is_coordinate(token: &str) -> bool {
            token.starts_with('~')
                || token.starts_with('^')
                || token.parse::<f64>().is_ok()
        }

        // Helper to parse 3 coordinate words into CommandPosition
        let parse_position = |x_str: &str, y_str: &str, z_str: &str| -> Result<CommandPosition, Box<temper_text::TextComponent>> {
            let input_str = format!("{} {} {}", x_str, y_str, z_str);
            let mut mock_ctx = temper_commands::CommandContext {
                input: temper_commands::CommandInput::of(input_str),
                command: ctx.command.clone(),
                sender: ctx.sender.clone(),
                state: ctx.state.clone(),
            };
            CommandPosition::parse(&mut mock_ctx)
        };

        // Helper to parse 1 word into EntityArgument
        let parse_entity = |token: &str| -> Result<EntityArgument, Box<temper_text::TextComponent>> {
            let mut mock_ctx = temper_commands::CommandContext {
                input: temper_commands::CommandInput::of(token.to_string()),
                command: ctx.command.clone(),
                sender: ctx.sender.clone(),
                state: ctx.state.clone(),
            };
            EntityArgument::parse(&mut mock_ctx)
        };

        match words.len() {
            1 => {
                // /tp <to-entity>
                let to_entity = parse_entity(&words[0])?;
                Ok(TpArgument {
                    from: None,
                    to: TeleportTarget::Entity(to_entity),
                })
            }
            2 => {
                // /tp <from-entity> <to-entity>
                let from_entity = parse_entity(&words[0])?;
                let to_entity = parse_entity(&words[1])?;
                Ok(TpArgument {
                    from: Some(from_entity),
                    to: TeleportTarget::Entity(to_entity),
                })
            }
            3 => {
                // /tp <x> <y> <z>
                if is_coordinate(&words[0]) && is_coordinate(&words[1]) && is_coordinate(&words[2]) {
                    let pos = parse_position(&words[0], &words[1], &words[2])?;
                    Ok(TpArgument {
                        from: None,
                        to: TeleportTarget::Position(pos),
                    })
                } else {
                    Err(temper_commands::arg::utils::parser_error("invalid coordinates"))
                }
            }
            4 => {
                // /tp <from-entity> <x> <y> <z>
                let from_entity = parse_entity(&words[0])?;
                if is_coordinate(&words[1]) && is_coordinate(&words[2]) && is_coordinate(&words[3]) {
                    let pos = parse_position(&words[1], &words[2], &words[3])?;
                    Ok(TpArgument {
                        from: Some(from_entity),
                        to: TeleportTarget::Position(pos),
                    })
                } else {
                    Err(temper_commands::arg::utils::parser_error("invalid coordinates"))
                }
            }
            _ => Err(temper_commands::arg::utils::parser_error("too many arguments")),
        }
    }

    fn primitive() -> temper_commands::arg::primitive::PrimitiveArgument {
        temper_commands::arg::primitive::PrimitiveArgument::greedy()
    }

    fn suggest(ctx: &mut temper_commands::CommandContext) -> Vec<temper_commands::Suggestion> {
        let input_str = ctx.input.remaining_input();

        // Consume input so has_remaining_input() is false
        ctx.input.read_string();
        while ctx.input.has_remaining_input() {
            ctx.input.read_string();
        }

        let words: Vec<&str> = input_str.split(' ').collect();

        fn is_coordinate(token: &str) -> bool {
            token.starts_with('~')
                || token.starts_with('^')
                || token.parse::<f64>().is_ok()
        }

        let mut suggest_entities = false;
        let mut suggest_coords = false;

        match words.len() {
            1 => {
                // /tp [word] -> could be from-entity or first coord of to-block
                suggest_entities = true;
                suggest_coords = true;
            }
            2 => {
                // /tp arg1 [word]
                if is_coordinate(words[0]) {
                    // /tp <x> [y]
                    suggest_coords = true;
                } else {
                    // /tp <from-entity> [word] -> could be to-entity or first coord of to-block
                    suggest_entities = true;
                    suggest_coords = true;
                }
            }
            3 => {
                // /tp arg1 arg2 [word]
                if is_coordinate(words[0]) {
                    // /tp <x> <y> [z]
                    suggest_coords = true;
                } else if is_coordinate(words[1]) {
                    // /tp <from-entity> <x> [y]
                    suggest_coords = true;
                }
            }
            4 => {
                // /tp arg1 arg2 arg3 [word]
                if !is_coordinate(words[0]) && is_coordinate(words[1]) {
                    // /tp <from-entity> <x> <y> [z]
                    suggest_coords = true;
                }
            }
            _ => {}
        }

        let current_word = words.last().copied().unwrap_or("");

        let mut suggestions = Vec::new();

        if suggest_coords {
            if current_word.is_empty() || current_word == "~" {
                suggestions.push(temper_commands::Suggestion::of("~"));
                suggestions.push(temper_commands::Suggestion::of("~ ~"));
                suggestions.push(temper_commands::Suggestion::of("~ ~ ~"));
            } else {
                suggestions.push(temper_commands::Suggestion::of(current_word.to_string()));
            }
        }

        if suggest_entities {
            let mut entity_suggestions = vec![
                temper_commands::Suggestion {
                    content: "@e".to_string(),
                    tooltip: Some(temper_nbt::NBT::new("Any Entity".into())),
                },
                temper_commands::Suggestion {
                    content: "@r".to_string(),
                    tooltip: Some(temper_nbt::NBT::new("Random Player".into())),
                },
                temper_commands::Suggestion {
                    content: "@a".to_string(),
                    tooltip: Some(temper_nbt::NBT::new("All Players".into())),
                },
            ];

            let state = ctx.state.clone();
            for kv in &state.clone().players.player_list {
                let (_, (uuid, name)) = kv.pair();
                entity_suggestions.push(temper_commands::Suggestion {
                    content: name.clone(),
                    tooltip: Some(temper_nbt::NBT::new(
                        ::uuid::Uuid::from_u128(*uuid)
                            .as_hyphenated()
                            .to_string()
                            .to_uppercase()
                            .into(),
                    )),
                });
            }

            for sug in entity_suggestions {
                if sug.content.to_lowercase().starts_with(&current_word.to_lowercase()) {
                    suggestions.push(sug);
                }
            }
        }

        suggestions
    }
}

#[command("tp")]
fn tp_command(
    #[sender] sender: Sender,
    #[arg] tp_arg: TpArgument,
    args: (
        Query<(&Rotation, &Position)>,
        MessageWriter<TeleportPlayer>,
        Query<(Entity, &Identity, Option<&PlayerMarker>)>,
    ),
) {
    let (mut query, mut tp_player_msg, resolve_q) = args;

    // The target entity that will be teleported (the 'from' entity).
    // If not specified, defaults to the command executor (the sender).
    let target_to_tp = match &tp_arg.from {
        Some(from_arg) => {
            let resolved = from_arg.resolve(resolve_q.iter());
            if resolved.len() != 1 {
                sender.send_message(
                    "You must specify exactly one target to teleport.".into(),
                    false,
                );
                return;
            }
            *resolved.first().unwrap()
        }
        None => {
            let Player(sender_e) = sender else {
                sender.send_message("You must specify a target when running this command from the server.".into(), false);
                return;
            };
            sender_e
        }
    };

    // The destination of the teleportation.
    match tp_arg.to {
        TeleportTarget::Position(pos) => {
            let Ok((rot, position)) = query.get_mut(target_to_tp) else {
                sender.send_message("Could not find the target's physical properties.".into(), false);
                return;
            };
            let resolved_pos = pos.resolve(position);

            tp_player_msg.write(TeleportPlayer {
                entity: target_to_tp,
                x: resolved_pos.x,
                y: resolved_pos.y,
                z: resolved_pos.z,
                vel_x: 0.0,
                vel_y: 0.0,
                vel_z: 0.0,
                yaw: rot.yaw,
                pitch: rot.pitch,
            });

            sender.send_message(format!("Teleported to ({}).", resolved_pos).into(), false);
        }
        TeleportTarget::Entity(to_ent_arg) => {
            if matches!(to_ent_arg, EntityArgument::AnyEntity | EntityArgument::AnyPlayer) {
                sender.send_message(
                    "Only one entity is allowed, but the provided selector can match multiple entities.".into(),
                    false,
                );
                return;
            }

            let resolved_dest = to_ent_arg.resolve(resolve_q.iter());
            if resolved_dest.len() != 1 {
                sender.send_message(
                    "You must specify exactly one destination entity.".into(),
                    false,
                );
                return;
            }
            let dest_entity = *resolved_dest.first().unwrap();

            if target_to_tp == dest_entity {
                sender.send_message("Cannot teleport an entity to itself.".into(), false);
                return;
            }

            // We need rotation of target_to_tp and position of dest_entity
            let Ok([(sender_rot, _), (_, target_pos)]) = query.get_many([target_to_tp, dest_entity]) else {
                sender.send_message("Could not find entity locations.".into(), false);
                return;
            };

            tp_player_msg.write(TeleportPlayer {
                entity: target_to_tp,
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
                vel_x: 0.0,
                vel_y: 0.0,
                vel_z: 0.0,
                yaw: sender_rot.yaw,
                pitch: sender_rot.pitch,
            });

            sender.send_message(
                format!("Teleported to the entity at {}.", target_pos).into(),
                false,
            );
        }
    }
}
