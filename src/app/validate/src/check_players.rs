use heed::byteorder::BigEndian;
use heed::types::{Bytes, U128};
use indicatif::ProgressStyle;
use std::process::exit;
use temper_components::player::offline_player_data::OfflinePlayerData;
use temper_state::ServerState;
use temper_storage::string_to_u128;
use type_hash::TypeHash;
use uuid::Uuid;

pub fn check_players(state: &ServerState) -> Result<(), String> {
    let env = state.world.storage_backend.env.lock();
    let txn = env
        .read_txn()
        .map_err(|e| format!("Failed to create read transaction: {}", e))?;

    if let Ok(Some(db)) = env.open_database::<U128<BigEndian>, Bytes>(&txn, Some("metadata")) {
        if let Ok(Some(player_format_hash)) = db.get(&txn, &string_to_u128("player-format-hash")) {
            let player_format_hash_u64 = u64::from_be_bytes(
                player_format_hash
                    .try_into()
                    .expect("Player format hash should be 8 bytes"),
            );
            if player_format_hash_u64 != OfflinePlayerData::type_hash() {
                eprintln!(
                    "Player format hash mismatch. Expected {}, got {}. This likely means that the player data format has changed since saving. \
                    If you have recently updated Temper you will have to go back to the older version until a world format converter is implemented.)",
                    OfflinePlayerData::type_hash(),
                    player_format_hash_u64
                );
                exit(1);
            }
        } else {
            eprintln!(
                "Could not find 'player-format-hash' in metadata. This likely means that the world was saved with an older version of Temper that did not include this metadata, or that the metadata is corrupted."
            );
            eprintln!(
                "If you have recently updated Temper you will have to go back to the older version until a world format converter is implemented.)"
            );
            exit(1);
        }
    } else {
        eprintln!(
            "Metadata database not found. This likely means that the world is empty and has no player data, so there is nothing to validate."
        );
        eprintln!(
            "Check that the world path is correct and that the world has been generated or used by a player at least once."
        );
        exit(1);
    }

    let Ok(db_opt) = env.open_database::<U128<BigEndian>, Bytes>(&txn, Some("player_data")) else {
        eprintln!(
            "Player data database not found. This likely means that the world is empty and has no player data, so there is nothing to validate."
        );
        eprintln!(
            "Check that the world path is correct and that the world has been generated or used by a player at least once."
        );
        exit(1);
    };
    let Some(db) = db_opt else {
        eprintln!(
            "Player data database not found. This likely means that the world has no player data to validate."
        );
        eprintln!(
            "Check that the world path is correct and that players have been saved at least once."
        );
        eprintln!("Player data validation will be skipped.");
        return Ok(());
    };
    let Ok(db_len) = db.len(&txn) else {
        eprintln!(
            "Failed to get the number of player entries in the database. This likely means that the database is corrupted."
        );
        exit(1);
    };

    let progress_bar = indicatif::ProgressBar::new(db_len);
    let progress_style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}/{eta_precise} eta] {bar:40.cyan/blue} {percent}% {pos}/{len} players validated")
        .unwrap();
    progress_bar.set_style(progress_style);

    for kv in db
        .iter(&txn)
        .expect("Failed to iterate over 'player_data' database")
    {
        let (key, value) = kv.expect("Failed to read key-value pair from 'player_data' database");
        let player_uuid = Uuid::from_u128(key);
        if value.is_empty() {
            eprintln!("Player {} has empty data, skipping.", player_uuid);
            progress_bar.inc(1);
            continue;
        }

        if let Err(e) = bitcode::decode::<OfflinePlayerData>(value) {
            progress_bar.finish();
            eprintln!("Player {} failed to decode: {}", player_uuid, e);
            eprintln!(
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
