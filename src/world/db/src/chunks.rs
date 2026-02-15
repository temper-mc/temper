use std::hash::Hasher;
use temper_config::server_config::get_global_config;
use temper_core::pos::ChunkPos;
use temper_storage::lmdb::LmdbBackend;
use temper_world_format::Chunk;
use temper_world_format::errors::WorldError;
use temper_world_format::errors::WorldError::CorruptedChunkData;
use tracing::warn;
use yazi::CompressionLevel;

pub fn save_chunk_internal(
    storage: &LmdbBackend,
    pos: ChunkPos,
    dimension: &str,
    chunk: &Chunk,
) -> Result<(), WorldError> {
    if !storage.table_exists("chunks".to_string())? {
        storage.create_table("chunks".to_string())?;
    }
    let as_bytes = yazi::compress(
        &bitcode::encode(chunk),
        yazi::Format::Zlib,
        CompressionLevel::BestSpeed,
    )?;
    let digest = create_key(dimension, pos);
    storage.upsert("chunks".to_string(), digest, as_bytes)?;
    Ok(())
}

pub fn load_chunk_internal(
    storage: &LmdbBackend,
    pos: ChunkPos,
    dimension: &str,
) -> Result<Chunk, WorldError> {
    let digest = create_key(dimension, pos);
    match storage.get("chunks".to_string(), digest)? {
        Some(compressed) => {
            let (data, checksum) = yazi::decompress(compressed.as_slice(), yazi::Format::Zlib)?;
            if get_global_config().database.verify_chunk_data {
                if let Some(expected_checksum) = checksum {
                    let real_checksum = yazi::Adler32::from_buf(data.as_slice()).finish();
                    if real_checksum != expected_checksum {
                        return Err(CorruptedChunkData(real_checksum, expected_checksum));
                    }
                } else {
                    warn!("Chunk data does not have a checksum, skipping verification.");
                }
            }
            let chunk: Chunk = bitcode::decode(&data)
                .map_err(|e| WorldError::BitcodeDecodeError(e.to_string()))?;
            Ok(chunk)
        }
        None => Err(WorldError::ChunkNotFound),
    }
}

pub fn load_chunk_batch_internal(
    storage: &LmdbBackend,
    coords: &[(ChunkPos, &str)],
) -> Result<Vec<Chunk>, WorldError> {
    let digests = coords
        .iter()
        .map(|&(pos, dim)| create_key(dim, pos))
        .collect();
    storage
        .batch_get("chunks".to_string(), digests)?
        .iter()
        .map(|chunk| match chunk {
            Some(compressed) => {
                let (data, checksum) = yazi::decompress(compressed, yazi::Format::Zlib)?;
                if get_global_config().database.verify_chunk_data {
                    if let Some(expected_checksum) = checksum {
                        let real_checksum = yazi::Adler32::from_buf(data.as_slice()).finish();
                        if real_checksum != expected_checksum {
                            return Err(CorruptedChunkData(real_checksum, expected_checksum));
                        }
                    } else {
                        warn!("Chunk data does not have a checksum, skipping verification.");
                    }
                }
                let chunk: Chunk = bitcode::decode(&data)
                    .map_err(|e| WorldError::BitcodeDecodeError(e.to_string()))?;
                Ok(chunk)
            }
            None => Err(WorldError::ChunkNotFound),
        })
        .collect()
}

pub fn chunk_exists_internal(
    storage: &LmdbBackend,
    pos: ChunkPos,
    dimension: &str,
) -> Result<bool, WorldError> {
    if !storage.table_exists("chunks".to_string())? {
        return Ok(false);
    }
    let digest = create_key(dimension, pos);
    Ok(storage.exists("chunks".to_string(), digest)?)
}

pub fn delete_chunk_internal(
    storage: &LmdbBackend,
    pos: ChunkPos,
    dimension: &str,
) -> Result<(), WorldError> {
    let digest = create_key(dimension, pos);
    storage.delete("chunks".to_string(), digest)?;
    Ok(())
}

pub fn sync_internal(storage: &LmdbBackend) -> Result<(), WorldError> {
    storage.flush()?;
    Ok(())
}

fn create_key(dimension: &str, pos: ChunkPos) -> u128 {
    let mut hasher = wyhash::WyHash::with_seed(0);
    hasher.write(dimension.as_bytes());
    hasher.write_u8(0xFF);
    let dim_hash = hasher.finish();
    (dim_hash as u128) << 96 | pos.pack() as u128
}
