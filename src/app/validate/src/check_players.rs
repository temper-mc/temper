use indicatif::ProgressStyle;
use std::process::exit;
use temper_components::player::offline_player_data::OfflinePlayerData;
use temper_state::ServerState;
use temper_storage::string_to_u128;
use tracing::{error, warn};
use type_hash::TypeHash;
use uuid::Uuid;

pub fn check_players(state: &ServerState) -> Result<(), String> {
    let backend = &state.world.chunks.storage_backend;
    let metadata_db = backend
        .database("metadata")
        .map_err(|e| format!("Failed to open metadata database: {}", e))?;
    let player_data_db = backend
        .database("player_data")
        .map_err(|e| format!("Failed to open player_data database: {}", e))?;
    let txn = backend
        .env()
        .read_txn()
        .map_err(|e| format!("Failed to create read transaction: {}", e))?;

    if let Some(db) = metadata_db {
        if let Ok(Some(player_format_hash)) = db.get(&txn, &string_to_u128("player-format-hash")) {
            let player_format_hash_u64 = u64::from_be_bytes(
                player_format_hash
                    .try_into()
                    .expect("Player format hash should be 8 bytes"),
            );
            if player_format_hash_u64 != OfflinePlayerData::type_hash() {
                error!(
                    "Player format hash mismatch. Expected {}, got {}. This likely means that the player data format has changed since saving. \
                    If you have recently updated Temper you will have to go back to the older version until a world format converter is implemented.)",
                    OfflinePlayerData::type_hash(),
                    player_format_hash_u64
                );
                exit(1);
            }
        } else {
            error!(
                "Could not find 'player-format-hash' in metadata. This likely means that the world was saved with an older version of Temper that did not include this metadata, or that the metadata is corrupted."
            );
            error!(
                "If you have recently updated Temper you will have to go back to the older version until a world format converter is implemented.)"
            );
            exit(1);
        }
    } else {
        warn!(
            "Metadata database not found. This likely means that the world is empty and has no player data, so there is nothing to validate."
        );
        warn!(
            "Check that the world path is correct and that the world has been generated or used by a player at least once."
        );
    }

    let Some(db) = player_data_db else {
        warn!(
            "Player data database not found. This likely means that the world has no player data to validate."
        );
        warn!(
            "Check that the world path is correct and that players have been saved at least once."
        );
        warn!("Player data validation will be skipped.");
        return Ok(());
    };
    let Ok(db_len) = db.len(&txn) else {
        error!(
            "Failed to get the number of player entries in the database. This likely means that the database is corrupted."
        );
        exit(1);
    };

    let progress_bar = indicatif::ProgressBar::new(db_len);
    let progress_style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}/{eta_precise} eta] {bar:40.magenta} {percent}% {pos}/{len} players validated")
        .unwrap();
    progress_bar.set_style(progress_style);

    for kv in db
        .iter(&txn)
        .expect("Failed to iterate over 'player_data' database")
    {
        let (key, value) = kv.expect("Failed to read key-value pair from 'player_data' database");
        let player_uuid = Uuid::from_u128(key);
        if value.is_empty() {
            warn!("Player {} has empty data, skipping.", player_uuid);
            progress_bar.inc(1);
            continue;
        }

        if let Err(e) = bitcode::deserialize::<OfflinePlayerData>(value) {
            progress_bar.finish();
            error!("Player {} failed to decode: {}", player_uuid, e);
            error!(
                "This generally means that the player data format has changed since saving. If you have \
                recently updated Temper you will have to go back to the older version until a world format converter is implemented.)"
            );
            exit(1);
        }

        progress_bar.inc(1);
    }

    progress_bar.finish();

    Ok(())
}
