use heed::byteorder::BigEndian;
use heed::types::{Bytes, U128};
use indicatif::ProgressStyle;
use std::process::exit;
use temper_core::pos::ChunkPos;
use temper_state::ServerState;
use temper_storage::string_to_u128;
use temper_world_format::Chunk;
use tracing::{error, warn};
use type_hash::TypeHash;

pub fn check_chunks(state: &ServerState) -> Result<(), String> {
    let env = state.world.storage_backend.env.lock();
    let txn = env
        .read_txn()
        .map_err(|e| format!("Failed to create read transaction: {}", e))?;

    if let Ok(Some(db)) = env.open_database::<U128<BigEndian>, Bytes>(&txn, Some("metadata")) {
        if let Ok(Some(chunk_format_hash)) = db.get(&txn, &string_to_u128("chunk-format-hash")) {
            let chunk_format_hash_str = u64::from_be_bytes(
                chunk_format_hash
                    .try_into()
                    .expect("Chunk format hash should be 8 bytes"),
            );
            if chunk_format_hash_str != Chunk::type_hash() {
                error!(
                    "Chunk format hash mismatch. Expected {}, got {}. This likely means that the chunk format has changed since saving. If you have \
                    recently updated Temper you will have to go back to the older version until a world format converter is implemented.)",
                    Chunk::type_hash(),
                    chunk_format_hash_str
                );
                exit(1);
            }
        } else {
            error!(
                "Could not find 'chunk-format-hash' in metadata. This likely means that the world was saved with an older version of Temper that did not include this metadata, or that the metadata is corrupted."
            );
            error!(
                "If you have recently updated Temper you will have to go back to the older version until a world format converter is implemented.)"
            );
            exit(1);
        }
    } else {
        error!(
            "Metadata database not found. This likely means that the world is empty and has no chunks, so there is nothing to validate."
        );
        error!(
            "Check that the world path is correct and that the world has been generated with at least one chunk."
        );
        exit(1);
    }

    let Ok(Some(db)) = env.open_database::<U128<BigEndian>, Bytes>(&txn, Some("chunks")) else {
        error!(
            "Could not open 'chunks' table. This likely means that the world is empty and has no chunks, so there is nothing to validate."
        );
        error!(
            "Check that the world path is correct and that the world has been generated with at least one chunk."
        );
        exit(1);
    };
    let Ok(db_len) = db.len(&txn) else {
        error!(
            "Failed to get the number of chunks in the database. This likely means that the database is corrupted."
        );
        exit(1);
    };
    let progress_bar = indicatif::ProgressBar::new(db_len);
    let progress_style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}/{eta_precise} eta] {bar:40.magenta} {percent}% {pos}/{len} chunks validated")
        .unwrap();
    progress_bar.set_style(progress_style);
    for kv in db
        .iter(&txn)
        .expect("Failed to iterate over 'chunks' database")
    {
        let (key, value) = kv.expect("Failed to read key-value pair from 'chunks' database");
        let decoded_key = get_coords_from_key(key);
        if value.is_empty() {
            warn!("Chunk {} has empty data, skipping.", decoded_key);
            progress_bar.inc(1);
            continue;
        }
        let Ok((data, checksum)) = yazi::decompress(value, yazi::Format::Zlib) else {
            progress_bar.finish();
            error!(
                "Chunk {} could not be decompressed. This likely means that the chunk data is corrupted.",
                decoded_key
            );
            exit(1);
        };
        if let Some(expected_checksum) = checksum {
            let real_checksum = yazi::Adler32::from_buf(data.as_slice()).finish();
            if real_checksum != expected_checksum {
                progress_bar.finish();
                error!(
                    "Chunk {} failed checksum verification. Expected {}, got {}. This likely means that the chunk data is corrupted.",
                    decoded_key, expected_checksum, real_checksum
                );
                exit(1);
            }
        }
        if let Err(e) = bitcode::decode::<Chunk>(&data) {
            progress_bar.finish();
            error!("Chunk {} failed to decode: {}", decoded_key, e);
            error!(
                "This generally means that the chunk format has changed since saving. If you have \
                recently updated Temper you will have to go back to the older version until a world format converter is implemented.)"
            );
            exit(1);
        }

        progress_bar.inc(1);
    }

    progress_bar.finish();

    Ok(())
}

fn get_coords_from_key(key: u128) -> ChunkPos {
    let pos_packed = (key & 0xFFFFFFFFFFFFFFFF) as u64;
    ChunkPos::unpack(pos_packed)
}
