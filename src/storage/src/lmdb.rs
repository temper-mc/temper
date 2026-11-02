use crate::errors::StorageError;
use heed;
use heed::byteorder::BigEndian;
use heed::types::{Bytes, U128};
use heed::{CommitOrAbort, Database, Env, EnvOpenOptions, WithoutTls};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// A database handle that has been committed, so it can outlive the transaction that opened it.
type Table = Database<'static, U128<BigEndian>, Bytes>;

#[derive(Debug, Clone)]
pub struct StorageBackend {
    env: Env<WithoutTls>,
    databases: Arc<RwLock<HashMap<String, Table>>>,
}

impl From<heed::Error> for StorageError {
    fn from(err: heed::Error) -> Self {
        match err {
            heed::Error::Io(e) => StorageError::GenericIoError(e),
            heed::Error::Encoding(e) => StorageError::WriteError(e.to_string()),
            heed::Error::Decoding(e) => StorageError::ReadError(e.to_string()),
            _ => StorageError::DatabaseError(err.to_string()),
        }
    }
}

impl StorageBackend {
    /// Looks up the handle for a table, opening it from disk if it isn't cached yet.
    pub fn database(&self, table: &str) -> Result<Option<Table>, StorageError> {
        if let Some(db) = self.databases.read().get(table) {
            return Ok(Some(*db));
        }

        // Not cached — it may still exist on disk from a previous run.
        let ro_txn = self.env.read_txn()?;
        let db = self
            .env
            .open_databases_and_commit(ro_txn, |dbo, ro_txn| {
                dbo.open_database::<U128<BigEndian>, Bytes>(ro_txn, Some(table))
                    .map(CommitOrAbort::<_, ()>::Commit)
            })?
            .unwrap_commit();
        let Some(db) = db else {
            return Ok(None);
        };

        self.databases.write().insert(table.to_string(), db);
        Ok(Some(db))
    }

    pub fn initialize(store_path: Option<PathBuf>, map_size: usize) -> Result<Self, StorageError>
    where
        Self: Sized,
    {
        let Some(checked_path) = store_path else {
            return Err(StorageError::InvalidPath);
        };
        if !checked_path.exists() {
            std::fs::create_dir_all(&checked_path)?;
        }
        let rounded_map_size = ((map_size as f64 / page_size::get() as f64).round()
            * page_size::get() as f64) as usize;
        unsafe {
            let env = EnvOpenOptions::new()
                .read_txn_without_tls()
                // Change this as more tables are needed.
                .max_dbs(3)
                .map_size(rounded_map_size)
                .open(checked_path)
                .map_err(|e| StorageError::DatabaseInitError(e.to_string()))?;

            Ok(StorageBackend {
                env,
                databases: Arc::new(RwLock::new(HashMap::new())),
            })
        }
    }

    pub fn insert(&self, table: String, key: u128, value: Vec<u8>) -> Result<(), StorageError> {
        let rw_txn = self.env.write_txn()?;
        let db = self
            .env
            .create_databases_and_commit(rw_txn, |dbo, rw_txn| {
                let db = dbo.create_database::<U128<BigEndian>, Bytes>(rw_txn, Some(&table))?;
                if db.get(rw_txn, &key)?.is_some() {
                    return Err(StorageError::KeyExists(key as u64));
                }
                db.put(rw_txn, &key, &value)?;
                Ok(CommitOrAbort::<_, ()>::Commit(db))
            })?
            .unwrap_commit();
        self.databases.write().insert(table, db);
        Ok(())
    }

    pub fn get(&self, table: String, key: u128) -> Result<Option<Vec<u8>>, StorageError> {
        let Some(db) = self.database(&table)? else {
            return Err(StorageError::TableError("Table not found".to_string()));
        };
        let ro_txn = self.env.read_txn()?;
        Ok(db.get(&ro_txn, &key)?.map(<[u8]>::to_vec))
    }

    pub fn delete(&self, table: String, key: u128) -> Result<(), StorageError> {
        let Some(db) = self.database(&table)? else {
            return Err(StorageError::TableError("Table not found".to_string()));
        };
        let mut rw_txn = self.env.write_txn()?;
        if db.get(&rw_txn, &key)?.is_none() {
            return Err(StorageError::KeyNotFound(key as u64));
        }
        db.delete(&mut rw_txn, &key)?;
        rw_txn.commit()?;
        Ok(())
    }

    pub fn update(&self, table: String, key: u128, value: Vec<u8>) -> Result<(), StorageError> {
        let Some(db) = self.database(&table)? else {
            return Err(StorageError::TableError("Table not found".to_string()));
        };
        let mut rw_txn = self.env.write_txn()?;
        if db.get(&rw_txn, &key)?.is_none() {
            return Err(StorageError::KeyNotFound(key as u64));
        }
        db.put(&mut rw_txn, &key, &value)?;
        rw_txn.commit()?;
        Ok(())
    }

    pub fn upsert(&self, table: String, key: u128, value: Vec<u8>) -> Result<bool, StorageError> {
        let Some(db) = self.database(&table)? else {
            return Err(StorageError::TableError("Table not found".to_string()));
        };
        let mut rw_txn = self.env.write_txn()?;
        db.put(&mut rw_txn, &key, &value)?;
        rw_txn.commit()?;
        Ok(true)
    }

    pub fn exists(&self, table: String, key: u128) -> Result<bool, StorageError> {
        let Some(db) = self.database(&table)? else {
            return Err(StorageError::TableError("Table not found".to_string()));
        };
        let ro_txn = self.env.read_txn()?;
        Ok(db.get(&ro_txn, &key)?.is_some())
    }

    pub fn table_exists(&self, table: String) -> Result<bool, StorageError> {
        Ok(self.database(&table)?.is_some())
    }

    pub fn flush(&self) -> Result<(), StorageError> {
        self.env.clear_stale_readers()?;
        self.env.force_sync()?;
        Ok(())
    }

    pub fn create_table(&self, table: String) -> Result<(), StorageError> {
        let rw_txn = self.env.write_txn()?;
        let db = self
            .env
            .create_databases_and_commit(rw_txn, |dbo, rw_txn| {
                dbo.create_database::<U128<BigEndian>, Bytes>(rw_txn, Some(&table))
                    .map(CommitOrAbort::<_, ()>::Commit)
            })?
            .unwrap_commit();
        self.databases.write().insert(table, db);
        Ok(())
    }

    pub fn close(&self) -> Result<(), StorageError> {
        self.flush()?;
        Ok(())
    }

    pub fn env(&self) -> &Env<WithoutTls> {
        &self.env
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::remove_dir_all;
    use std::hash::Hasher;
    use tempfile::tempdir;

    fn hash_2_to_u128(a: u64, b: u64) -> u128 {
        let mut hasher = wyhash::WyHash::with_seed(0);
        hasher.write_u64(a);
        hasher.write_u64(b);
        u128::from(hasher.finish())
    }

    #[test]
    fn test_write() {
        let path = tempdir().unwrap().keep();
        {
            let backend =
                StorageBackend::initialize(Some(path.clone()), 10 * 1024 * 1024 * 1024).unwrap();
            backend.create_table("test_table".to_string()).unwrap();
            let key = 12345678901234567890u128;
            let value = vec![1, 2, 3, 4, 5];
            backend
                .insert("test_table".to_string(), key, value.clone())
                .unwrap();
            let retrieved_value = backend.get("test_table".to_string(), key).unwrap();
            assert_eq!(retrieved_value, Some(value));
        }
        remove_dir_all(path).unwrap();
    }

    #[test]
    fn test_concurrent_write() {
        let path = tempdir().unwrap().keep();
        {
            let backend =
                StorageBackend::initialize(Some(path.clone()), 10 * 1024 * 1024 * 1024).unwrap();
            backend.create_table("test_table".to_string()).unwrap();
            let mut threads = vec![];
            for thread_iter in 0..10 {
                let handle = std::thread::spawn({
                    let backend = backend.clone();
                    move || {
                        for iter in 0..100 {
                            let key = hash_2_to_u128(iter, thread_iter);
                            let value = vec![rand::random::<u8>(); 10];
                            backend
                                .insert("test_table".to_string(), key, value)
                                .unwrap();
                        }
                    }
                });
                threads.push(handle);
            }
            for handle in threads {
                handle.join().unwrap();
            }
        }
        remove_dir_all(path).unwrap();
    }

    #[test]
    fn test_concurrent_read() {
        let path = tempdir().unwrap().keep();
        {
            let backend =
                StorageBackend::initialize(Some(path.clone()), 10 * 1024 * 1024 * 1024).unwrap();
            backend.create_table("test_table".to_string()).unwrap();
            for thread_iter in 0..10 {
                for iter in 0..100 {
                    let value = vec![rand::random::<u8>(); 10];
                    let key = hash_2_to_u128(iter, thread_iter);
                    backend
                        .insert("test_table".to_string(), key, value)
                        .unwrap();
                }
            }
            let mut threads = vec![];
            for thread_iter in 0..10 {
                let handle = std::thread::spawn({
                    let backend = backend.clone();
                    move || {
                        for iter in 0..100 {
                            let key = hash_2_to_u128(iter, thread_iter);
                            let _ = backend.get("test_table".to_string(), key).unwrap();
                        }
                    }
                });
                threads.push(handle);
            }
            for handle in threads {
                handle.join().unwrap();
            }
        }
        remove_dir_all(path).unwrap();
    }

    /// Not a correctness test — a contention benchmark. Reads are lock-free in
    /// LMDB, so this should scale with cores; if it doesn't, something is
    /// serializing them.
    #[test]
    fn bench_concurrent_read_contention() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let path = tempdir().unwrap().keep();
        {
            let backend =
                StorageBackend::initialize(Some(path.clone()), 10 * 1024 * 1024 * 1024).unwrap();
            backend.create_table("test_table".to_string()).unwrap();

            for thread_iter in 0..10u64 {
                for iter in 0..100u64 {
                    let value = vec![rand::random::<u8>(); 4096];
                    let key = hash_2_to_u128(iter, thread_iter);
                    backend
                        .insert("test_table".to_string(), key, value)
                        .unwrap();
                }
            }

            // Keep a writer busy for the duration, the way chunk_unloader and
            // world_sync are while generation reads.
            let stop = Arc::new(AtomicBool::new(false));
            let writer = std::thread::spawn({
                let backend = backend.clone();
                let stop = Arc::clone(&stop);
                move || {
                    let mut n = 0u64;
                    while !stop.load(Ordering::Relaxed) {
                        let value = vec![rand::random::<u8>(); 4096];
                        let key = hash_2_to_u128(n % 100, n % 10);
                        backend
                            .upsert("test_table".to_string(), key, value)
                            .unwrap();
                        n += 1;
                    }
                    n
                }
            });

            let start = std::time::Instant::now();
            let mut threads = vec![];
            for thread_iter in 0..24u64 {
                let handle = std::thread::spawn({
                    let backend = backend.clone();
                    move || {
                        for iter in 0..1000u64 {
                            // Mirrors ensure_chunk: table check, then a miss.
                            let _ = backend.table_exists("test_table".to_string()).unwrap();
                            let key = hash_2_to_u128(iter + 1_000_000, thread_iter);
                            let _ = backend.exists("test_table".to_string(), key).unwrap();
                        }
                    }
                });
                threads.push(handle);
            }
            for handle in threads {
                handle.join().unwrap();
            }
            let elapsed = start.elapsed();

            stop.store(true, Ordering::Relaxed);
            let writes = writer.join().unwrap();
            println!("24000 read pairs took {elapsed:?} with {writes} concurrent writes");
        }
        remove_dir_all(path).unwrap();
    }

    /// Several threads racing to create and populate the *same* new table.
    /// `mdb_dbi_open` is documented as unsafe to call concurrently; heed is
    /// supposed to guard this, and `insert` creates the database every time.
    #[test]
    fn concurrent_table_creation_is_safe() {
        let path = tempdir().unwrap().keep();
        {
            let backend =
                StorageBackend::initialize(Some(path.clone()), 1024 * 1024 * 1024).unwrap();

            let mut threads = vec![];
            for thread_iter in 0..16u64 {
                threads.push(std::thread::spawn({
                    let backend = backend.clone();
                    move || {
                        for iter in 0..50u64 {
                            let key = hash_2_to_u128(iter, thread_iter);
                            backend
                                .insert("racy_table".to_string(), key, vec![thread_iter as u8; 32])
                                .unwrap();
                        }
                    }
                }));
            }
            for handle in threads {
                handle.join().expect("no thread should panic");
            }

            assert!(backend.table_exists("racy_table".to_string()).unwrap());
            for thread_iter in 0..16u64 {
                for iter in 0..50u64 {
                    let key = hash_2_to_u128(iter, thread_iter);
                    let value = backend.get("racy_table".to_string(), key).unwrap();
                    assert_eq!(value, Some(vec![thread_iter as u8; 32]));
                }
            }
        }
        remove_dir_all(path).unwrap();
    }

    /// Readers and writers on overlapping keys. Every read must return a value
    /// that was written at some point, never a torn or partial one.
    #[test]
    fn concurrent_mixed_workload_stays_consistent() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let path = tempdir().unwrap().keep();
        {
            let backend =
                StorageBackend::initialize(Some(path.clone()), 1024 * 1024 * 1024).unwrap();
            backend.create_table("mixed".to_string()).unwrap();

            // Every value is 256 copies of a single marker byte, so a torn
            // read is detectable without knowing which write won.
            for key in 0..100u128 {
                backend
                    .insert("mixed".to_string(), key, vec![0u8; 256])
                    .unwrap();
            }

            let stop = Arc::new(AtomicBool::new(false));
            let mut writers = vec![];
            for marker in 1..=4u8 {
                writers.push(std::thread::spawn({
                    let backend = backend.clone();
                    let stop = Arc::clone(&stop);
                    move || {
                        let mut n = 0u128;
                        while !stop.load(Ordering::Relaxed) {
                            backend
                                .upsert("mixed".to_string(), n % 100, vec![marker; 256])
                                .unwrap();
                            n += 1;
                        }
                    }
                }));
            }

            let mut readers = vec![];
            for _ in 0..16 {
                readers.push(std::thread::spawn({
                    let backend = backend.clone();
                    move || {
                        for _ in 0..2000 {
                            for key in 0..100u128 {
                                let value = backend
                                    .get("mixed".to_string(), key)
                                    .unwrap()
                                    .expect("key should always exist");
                                assert_eq!(value.len(), 256, "value was torn");
                                let marker = value[0];
                                assert!(marker <= 4, "value contains garbage");
                                assert!(
                                    value.iter().all(|b| *b == marker),
                                    "value mixes two different writes"
                                );
                            }
                        }
                    }
                }));
            }

            for handle in readers {
                handle.join().expect("no reader should panic");
            }
            stop.store(true, Ordering::Relaxed);
            for handle in writers {
                handle.join().expect("no writer should panic");
            }
        }
        remove_dir_all(path).unwrap();
    }

    /// LMDB's reader table defaults to 126 slots. The server runs more workers
    /// than that, so confirm concurrent reads past the limit don't fail.
    #[test]
    fn many_concurrent_readers() {
        let path = tempdir().unwrap().keep();
        {
            let backend =
                StorageBackend::initialize(Some(path.clone()), 1024 * 1024 * 1024).unwrap();
            backend.create_table("readers".to_string()).unwrap();
            backend
                .insert("readers".to_string(), 1u128, vec![7u8; 64])
                .unwrap();

            let mut threads = vec![];
            for _ in 0..200 {
                threads.push(std::thread::spawn({
                    let backend = backend.clone();
                    move || {
                        for _ in 0..100 {
                            let value = backend.get("readers".to_string(), 1u128).unwrap();
                            assert_eq!(value, Some(vec![7u8; 64]));
                        }
                    }
                }));
            }
            for handle in threads {
                handle.join().expect("no reader should panic");
            }
        }
        remove_dir_all(path).unwrap();
    }

    /// The handle cache starts empty on a fresh process, so `database()` has to
    /// find tables that already exist on disk.
    #[test]
    fn tables_resolve_after_reopen() {
        let path = tempdir().unwrap().keep();
        {
            let backend =
                StorageBackend::initialize(Some(path.clone()), 1024 * 1024 * 1024).unwrap();
            backend.create_table("persisted".to_string()).unwrap();
            backend
                .insert("persisted".to_string(), 42u128, vec![9u8; 16])
                .unwrap();
            backend.flush().unwrap();
        }
        {
            let backend =
                StorageBackend::initialize(Some(path.clone()), 1024 * 1024 * 1024).unwrap();
            assert!(backend.table_exists("persisted".to_string()).unwrap());
            assert!(!backend.table_exists("never_created".to_string()).unwrap());
            assert_eq!(
                backend.get("persisted".to_string(), 42u128).unwrap(),
                Some(vec![9u8; 16])
            );
        }
        remove_dir_all(path).unwrap();
    }
}
