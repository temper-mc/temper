use crate::entity_types::EntityTypeEnum;
use bevy_ecs::prelude::Component;
use temper_components::combat::CombatProperties;
use temper_components::entity_identity::Identity;
use temper_components::last_synced_position::LastSyncedPosition;
use temper_components::metadata::EntityMetadata;
use temper_components::player::grounded::OnGround;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_components::player::velocity::Velocity;
use temper_components::spawn::SpawnProperties;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct MobKind(pub EntityTypeEnum);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MobProfile {
    Ground,
    CollisionOnly,
    GravityNoDrag,
}

pub struct StandardMobParts<'a> {
    pub identity: &'a Identity,
    pub metadata: &'a EntityMetadata,
    pub combat: &'a CombatProperties,
    pub spawn: &'a SpawnProperties,
    pub position: &'a Position,
    pub rotation: &'a Rotation,
    pub velocity: &'a Velocity,
    pub on_ground: &'a OnGround,
    pub last_synced_position: &'a LastSyncedPosition,
}

pub trait MobDefinition {
    type Bundle;

    const KIND: EntityTypeEnum;
    const PROFILE: MobProfile;

    fn new(position: Position) -> Self::Bundle;
    fn clone_bundle(bundle: &Self::Bundle) -> Self::Bundle;
    fn from_standard_parts(parts: StandardMobParts<'_>) -> Self::Bundle;
    fn identity(bundle: &Self::Bundle) -> &Identity;
    fn position(bundle: &Self::Bundle) -> Position;
}

#[macro_export]
macro_rules! define_mob {
    (
        $definition:ident {
            kind = $kind:ident,
            vanilla = $vanilla:ident,
            bundle = $bundle:ident,
            marker = $marker:path,
            profile = $profile:ident,
            runtime = {
                $(
                    $runtime_field:ident: $runtime_ty:path => $runtime_init:ident
                ),+ $(,)?
            },
            persisted = {
                identity: $identity:path => clone,
                metadata: $metadata:path => copy,
                combat: $combat:path => copy,
                spawn: $spawn:path => clone,
                position: $position:path => copy,
                rotation: $rotation:path => copy,
                velocity: $velocity:path => copy,
                on_ground: $on_ground:path => copy,
                last_synced_position: $last_synced_position:path => copy $(,)?
            } $(,)?
        }
    ) => {
        compile_error!(
            "define_mob! runtime components are defaulted automatically; write `field: Type` instead of `field: Type => default`"
        );
    };

    (
        $definition:ident {
            kind = $kind:ident,
            vanilla = $vanilla:ident,
            bundle = $bundle:ident,
            marker = $marker:path,
            profile = $profile:ident,
            runtime = {
                $(
                    $runtime_field:ident: $runtime_ty:path
                ),* $(,)?
            },
            persisted = {
                identity: $identity:path => clone,
                metadata: $metadata:path => copy,
                combat: $combat:path => copy,
                spawn: $spawn:path => clone,
                position: $position:path => copy,
                rotation: $rotation:path => copy,
                velocity: $velocity:path => copy,
                on_ground: $on_ground:path => copy,
                last_synced_position: $last_synced_position:path => copy $(,)?
            } $(,)?
        }
    ) => {
        use bevy_ecs::prelude::Bundle;
        use temper_data::generated::entities::EntityType as VanillaEntityType;

        fn de_meta<'de, D>(_: D) -> Result<$metadata, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(<$metadata>::from_vanilla(&VanillaEntityType::$vanilla))
        }

        fn de_spawn<'de, D>(_: D) -> Result<$spawn, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(<$spawn>::from_vanilla(&VanillaEntityType::$vanilla))
        }

        #[derive(Bundle, serde::Serialize, serde::Deserialize)]
        pub struct $bundle {
            pub identity: $identity,
            #[serde(skip_serializing)]
            #[serde(deserialize_with = "de_meta")]
            pub metadata: $metadata,
            pub combat: $combat,
            #[serde(skip_serializing)]
            #[serde(deserialize_with = "de_spawn")]
            pub spawn: $spawn,
            pub position: $position,
            pub rotation: $rotation,
            pub velocity: $velocity,
            pub on_ground: $on_ground,
            pub last_synced_position: $last_synced_position,
            $(
                #[serde(skip)]
                pub $runtime_field: $runtime_ty,
            )*
            #[serde(skip)]
            pub game_id: temper_components::game_id::GameID,
        }

        impl $bundle {
            pub fn new(position: $position) -> Self {
                let metadata = <$metadata>::from_vanilla(&VanillaEntityType::$vanilla);
                let combat = <$combat>::from_metadata(&metadata);
                let spawn = <$spawn>::from_metadata(&metadata);

                Self {
                    identity: <$identity>::new(None),
                    metadata,
                    combat,
                    spawn,
                    rotation: <$rotation>::default(),
                    velocity: <$velocity>::zero(),
                    on_ground: $on_ground(false),
                    last_synced_position: <$last_synced_position>::from_position(&position),
                    position,
                    $(
                        $runtime_field: <$runtime_ty>::default(),
                    )*
                    game_id: temper_components::game_id::GameID::new()
                }
            }

            pub fn with_rotation(position: $position, rotation: $rotation) -> Self {
                let mut bundle = Self::new(position);
                bundle.rotation = rotation;
                bundle
            }

            pub fn profile(&self) -> $crate::mob_definition::MobProfile {
                $crate::mob_definition::MobProfile::$profile
            }
        }

        pub struct $definition;

        impl $definition {
            pub fn spawn_standard(
                commands: &mut bevy_ecs::prelude::Commands<'_, '_>,
                bundle: $bundle,
            ) -> bevy_ecs::prelude::Entity {
                let last_chunk =
                    temper_components::last_chunk_pos::LastChunkPos::new(bundle.position.chunk());
                let entity = commands
                    .spawn((
                        bundle,
                        $marker,
                        $crate::mob_definition::MobKind(
                            $crate::entity_types::EntityTypeEnum::$kind,
                        ),
                        last_chunk,
                    ))
                    .id();

                match $crate::mob_definition::MobProfile::$profile {
                    $crate::mob_definition::MobProfile::Ground => {
                        commands.entity(entity).insert((
                            $crate::markers::HasGravity,
                            $crate::markers::HasCollisions,
                            $crate::markers::HasWaterDrag,
                        ));
                    }
                    $crate::mob_definition::MobProfile::CollisionOnly => {
                        commands
                            .entity(entity)
                            .insert($crate::markers::HasCollisions);
                    }
                    $crate::mob_definition::MobProfile::GravityNoDrag => {
                        commands
                            .entity(entity)
                            .insert(($crate::markers::HasGravity, $crate::markers::HasCollisions));
                    }
                }

                entity
            }
        }

        impl $crate::mob_definition::MobDefinition for $definition {
            type Bundle = $bundle;

            const KIND: $crate::entity_types::EntityTypeEnum =
                $crate::entity_types::EntityTypeEnum::$kind;
            const PROFILE: $crate::mob_definition::MobProfile =
                $crate::mob_definition::MobProfile::$profile;

            fn new(position: $position) -> Self::Bundle {
                Self::Bundle::new(position)
            }

            fn clone_bundle(bundle: &Self::Bundle) -> Self::Bundle {
                Self::Bundle {
                    identity: bundle.identity.clone(),
                    metadata: bundle.metadata,
                    combat: bundle.combat,
                    spawn: bundle.spawn.clone(),
                    position: bundle.position,
                    rotation: bundle.rotation,
                    velocity: bundle.velocity,
                    on_ground: bundle.on_ground,
                    last_synced_position: bundle.last_synced_position,
                    $(
                        $runtime_field: <$runtime_ty>::default(),
                    )*
                    game_id: bundle.game_id,
                }
            }

            fn from_standard_parts(
                parts: $crate::mob_definition::StandardMobParts<'_>,
            ) -> Self::Bundle {
                Self::Bundle {
                    identity: parts.identity.clone(),
                    metadata: *parts.metadata,
                    combat: *parts.combat,
                    spawn: parts.spawn.clone(),
                    position: *parts.position,
                    rotation: *parts.rotation,
                    velocity: *parts.velocity,
                    on_ground: *parts.on_ground,
                    last_synced_position: *parts.last_synced_position,
                    $(
                        $runtime_field: <$runtime_ty>::default(),
                    )*
                    game_id: temper_components::game_id::GameID::new(),
                }
            }

            fn identity(bundle: &Self::Bundle) -> &$identity {
                &bundle.identity
            }

            fn position(bundle: &Self::Bundle) -> $position {
                bundle.position
            }
        }
    };

    ($definition:ident { $($tokens:tt)* }) => {
        compile_error!(
            "invalid define_mob! syntax; expected `kind`, `vanilla`, `bundle`, `marker`, `profile`, `runtime`, and `persisted` sections"
        );
    };

    ($($tokens:tt)*) => {
        compile_error!(
            "invalid define_mob! invocation; expected `define_mob! { MobDefinition { ... } }`"
        );
    };
}

#[macro_export]
macro_rules! define_mob_registry {
    (
        $(
            $variant:ident = $definition:path
        ),+ $(,)?
    ) => {
        pub enum MobBundle {
            $(
                $variant(<$definition as $crate::mob_definition::MobDefinition>::Bundle),
            )+
        }

        impl Clone for MobBundle {
            fn clone(&self) -> Self {
                match self {
                    $(
                        Self::$variant(bundle) => {
                            Self::$variant(
                                <$definition as $crate::mob_definition::MobDefinition>::clone_bundle(bundle),
                            )
                        }
                    )+
                }
            }
        }

        impl MobBundle {
            pub fn all_kinds() -> &'static [$crate::entity_types::EntityTypeEnum] {
                const KINDS: &[$crate::entity_types::EntityTypeEnum] = &[
                    $(
                        <$definition as $crate::mob_definition::MobDefinition>::KIND,
                    )+
                ];

                KINDS
            }

            pub fn deserialize(
                kind: $crate::entity_types::EntityTypeEnum,
                data: &[u8],
            ) -> Option<Self> {
                match kind {
                    $(
                        <$definition as $crate::mob_definition::MobDefinition>::KIND => {
                            bitcode::deserialize(data).ok().map(Self::$variant)
                        }
                    )+
                }
            }

            pub fn new(
                kind: $crate::entity_types::EntityTypeEnum,
                position: temper_components::player::position::Position,
            ) -> Self {
                match kind {
                    $(
                        <$definition as $crate::mob_definition::MobDefinition>::KIND => {
                            Self::$variant(
                                <$definition as $crate::mob_definition::MobDefinition>::new(position),
                            )
                        }
                    )+
                }
            }

            pub fn serialize_for_chunk(&self) -> Vec<u8> {
                match self {
                    $(
                        Self::$variant(bundle) =>
                            bitcode::serialize(bundle).expect("Failed to serialize mob bundle"),
                    )+
                }
            }

            pub fn from_standard_parts(
                kind: $crate::entity_types::EntityTypeEnum,
                parts: $crate::mob_definition::StandardMobParts<'_>,
            ) -> Self {
                match kind {
                    $(
                        <$definition as $crate::mob_definition::MobDefinition>::KIND => {
                            Self::$variant(
                                <$definition as $crate::mob_definition::MobDefinition>::from_standard_parts(parts),
                            )
                        }
                    )+
                }
            }

            pub fn kind(&self) -> $crate::entity_types::EntityTypeEnum {
                match self {
                    $(
                        Self::$variant(_) =>
                            <$definition as $crate::mob_definition::MobDefinition>::KIND,
                    )+
                }
            }

            pub fn profile(&self) -> $crate::mob_definition::MobProfile {
                match self {
                    $(
                        Self::$variant(_) =>
                            <$definition as $crate::mob_definition::MobDefinition>::PROFILE,
                    )+
                }
            }

            pub fn identity(&self) -> &temper_components::entity_identity::Identity {
                match self {
                    $(
                        Self::$variant(bundle) =>
                            <$definition as $crate::mob_definition::MobDefinition>::identity(bundle),
                    )+
                }
            }

            pub fn position(&self) -> temper_components::player::position::Position {
                match self {
                    $(
                        Self::$variant(bundle) =>
                            <$definition as $crate::mob_definition::MobDefinition>::position(bundle),
                    )+
                }
            }

            pub fn spawn_standard(
                self,
                commands: &mut bevy_ecs::prelude::Commands<'_, '_>,
            ) -> bevy_ecs::prelude::Entity {
                match self {
                    $(
                        Self::$variant(bundle) => <$definition>::spawn_standard(commands, bundle),
                    )+
                }
            }
        }
    };
}
