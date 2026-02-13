#![expect(clippy::type_complexity)]
use bevy_ecs::prelude::{Query, Res, ResMut};
use temper_components::active_effects::ActiveEffects;
use temper_components::health::Health;
use temper_components::player::abilities::PlayerAbilities;
use temper_components::player::experience::Experience;
use temper_components::player::gamemode::GameModeComponent;
use temper_components::player::gameplay_state::ender_chest::EnderChest;
use temper_components::player::hunger::Hunger;
use temper_components::player::offline_player_data::OfflinePlayerData;
use temper_components::player::player_identity::PlayerIdentity;
use temper_components::player::position::Position;
use temper_components::player::rotation::Rotation;
use temper_inventories::inventory::Inventory;
use temper_resources::world_sync_tracker::WorldSyncTracker;
use temper_state::GlobalStateResource;

pub fn sync_world(
    player_query: Query<(
        &PlayerIdentity,
        &PlayerAbilities,
        &GameModeComponent,
        &Position,
        &Rotation,
        &Inventory,
        &Health,
        &Hunger,
        &Experience,
        &EnderChest,
        &ActiveEffects,
    )>,
    state: Res<GlobalStateResource>,
    mut last_synced: ResMut<WorldSyncTracker>,
) {
    if state.0.shut_down.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    // Always schedule a sync; frequency is handled by the schedule period.
    state.0.world.sync().expect("Failed to sync world");

    for (
        identity,
        abilities,
        gamemode,
        position,
        rotation,
        inventory,
        health,
        hunger,
        experience,
        ender_chest,
        active_effects,
    ) in player_query.iter()
    {
        let data = OfflinePlayerData {
            abilities: *abilities,
            gamemode: gamemode.0,
            position: (*position).into(),
            rotation: *rotation,
            inventory: inventory.clone(),
            health: *health,
            hunger: *hunger,
            experience: *experience,
            ender_chest: ender_chest.clone(),
            active_effects: active_effects.clone(),
        };
        state
            .0
            .world
            .save_player_data(identity.uuid, &data)
            .expect("Failed to save player data");
    }

    last_synced.last_synced = std::time::Instant::now();
}
