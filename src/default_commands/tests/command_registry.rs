use bevy_ecs::prelude::World;
use temper_command_infra::{
    CommandGraph, CommandNodeKind, CommandPathSegment, CommandRegistry, EntityProperties,
    ParserKind, ParserProperties, ResourceProperties, SuggestionInput, suggest_command_arg,
};
use temper_protocol::outgoing::commands::{CommandsPacket, PrimitiveArgumentType};

#[test]
fn default_commands_register_metadata() {
    temper_default_commands::init();

    let registry = CommandRegistry::from_static_commands();
    let paths = registry.paths_for_player(World::new().spawn_empty().id());

    assert!(paths.iter().any(|path| path.root == "tp"));
    assert!(paths.iter().any(|path| path.root == "stop"));
    assert!(paths.iter().any(|path| path.root == "echo"));
    assert!(paths.iter().any(|path| path.root == "time"));
    assert!(paths.iter().any(|path| path.root == "bossbar"));
    assert!(paths.iter().any(|path| path.root == "gamemode"));
    assert!(paths.iter().any(|path| path.root == "op"));
    assert!(paths.iter().any(|path| path.root == "summon"));
    assert!(paths.iter().any(|path| path.root == "spawn"));
    assert!(paths.iter().any(|path| path.root == "noise"));
    assert!(paths.iter().any(|path| path.root == "noises"));
    assert!(paths.iter().any(|path| path.root == "damage"));

    let summon = registry
        .commands()
        .iter()
        .find(|command| command.name == "summon")
        .unwrap();
    assert!(summon.aliases.contains(&"spawn"));
    assert!(summon.matches_root("spawn"));

    let summon_entity_spec = paths
        .iter()
        .filter(|path| path.root == "summon")
        .filter_map(|path| path.segments.first())
        .find_map(|segment| match segment {
            CommandPathSegment::Argument { spec, .. } => Some(*spec),
            _ => None,
        })
        .unwrap();
    assert_eq!(summon_entity_spec.parser, ParserKind::Resource);
    assert_eq!(
        summon_entity_spec.properties,
        Some(ParserProperties::Resource(ResourceProperties {
            registry: "minecraft:entity_type",
        }))
    );

    let op_target_spec = paths
        .iter()
        .filter(|path| path.root == "op")
        .filter_map(|path| path.segments.first())
        .find_map(|segment| match segment {
            CommandPathSegment::Argument { spec, .. } => Some(*spec),
            _ => None,
        })
        .unwrap();
    assert_eq!(op_target_spec.parser, ParserKind::Entity);
    assert_eq!(
        op_target_spec.properties,
        Some(ParserProperties::Entity(EntityProperties {
            single: false,
            players_only: false,
        }))
    );

    let stop = paths.iter().find(|path| path.root == "stop").unwrap();
    let echo = paths.iter().find(|path| path.root == "echo").unwrap();

    assert!(stop.segments.is_empty());
    assert_eq!(echo.segments.len(), 1);
    assert!(matches!(
        echo.segments[0],
        CommandPathSegment::Argument {
            name: "message",
            ..
        }
    ));

    assert!(paths.iter().any(|path| path.root == "time"
        && matches!(
            path.segments.as_slice(),
            [
                CommandPathSegment::Literal { name: "set", .. },
                CommandPathSegment::Literal { name: "day", .. }
            ]
        )));
    assert!(paths.iter().any(|path| path.root == "time"
        && matches!(
            path.segments.as_slice(),
            [
                CommandPathSegment::Literal { name: "set", .. },
                CommandPathSegment::Literal { name: "d", .. }
            ]
        )));
    assert!(paths.iter().any(|path| path.root == "time"
        && matches!(
            path.segments.as_slice(),
            [
                CommandPathSegment::Literal { name: "set", .. },
                CommandPathSegment::Argument { name: "value", .. }
            ]
        )));
}

#[test]
fn default_gamemode_arg_registers_suggestions() {
    temper_default_commands::init();

    let registry = CommandRegistry::from_static_commands();
    let paths = registry.paths_for_player(World::new().spawn_empty().id());
    let provider = paths
        .iter()
        .filter(|path| path.root == "gamemode")
        .filter_map(|path| path.segments.first())
        .find_map(|segment| match segment {
            CommandPathSegment::Argument { spec, .. } => spec.server_suggestions,
            _ => None,
        })
        .unwrap();

    let mut world = World::new();
    let source = world.spawn_empty().id();
    let suggestions = suggest_command_arg(
        provider,
        &mut world,
        SuggestionInput {
            full_input: "/gamemode ",
            current_token: "",
            source,
        },
    )
    .unwrap();

    assert_eq!(
        suggestions,
        vec!["survival", "creative", "adventure", "spectator"]
    );
}

#[test]
fn default_tp_graph_uses_client_handled_entity_and_position_parsers() {
    temper_default_commands::init();

    let registry = CommandRegistry::from_static_commands();
    let graph =
        CommandGraph::from_paths(&registry.paths_for_player(World::new().spawn_empty().id()));
    let root = &graph.nodes[graph.root_idx];
    let tp_idx = root
        .children
        .iter()
        .copied()
        .find(|idx| graph.nodes[*idx].name.as_deref() == Some("tp"))
        .unwrap();
    let tp = &graph.nodes[tp_idx];

    let first_child = &graph.nodes[tp.children[0]];
    assert_eq!(first_child.kind, CommandNodeKind::Argument);
    assert_eq!(first_child.name.as_deref(), Some("target"));
    assert_eq!(
        first_child.argument.map(|argument| argument.parser),
        Some(ParserKind::Entity)
    );
    assert_eq!(
        first_child
            .argument
            .and_then(|argument| argument.protocol_suggestions),
        None
    );

    let packet = CommandsPacket::from_command_infra_graph(&graph);
    let packet_tp = &packet.graph.data[tp_idx];
    let packet_first_child_idx = packet_tp.children.data[0].0 as usize;
    let packet_first_child = &packet.graph.data[packet_first_child_idx];

    assert_eq!(
        packet_first_child.parser_id,
        Some(PrimitiveArgumentType::Entity)
    );
    assert_eq!(packet_first_child.suggestions_type, None);

    let has_client_position_branch = packet_tp.children.data.iter().any(|child_idx| {
        let child = &packet.graph.data[child_idx.0 as usize];
        child.parser_id == Some(PrimitiveArgumentType::Vec3) && child.suggestions_type.is_none()
    });

    assert!(
        has_client_position_branch,
        "tp should expose a plain vec3 branch for client coordinate suggestions"
    );
}
