use bevy_ecs::query::QueryEntityError;
use ionic_core::errors::CoreError;
use ionic_general_purpose::paths::RootPathError;
use ionic_inventories::errors::InventoryError;
use ionic_plugins::errors::PluginsError;
use ionic_protocol::errors::NetError;
use ionic_storage::errors::StorageError;
use ionic_utils::errors::UtilsError;
use ionic_world::world_gen::errors::WorldGenError;
use ionic_world_format::errors::WorldError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BinaryError {
    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("QueryError error: {0}")]
    QueryError(#[from] QueryEntityError),

    #[error("Net error: {0}")]
    Net(#[from] NetError),

    #[error("Plugins error: {0}")]
    Plugins(#[from] PluginsError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Utils error: {0}")]
    Utils(#[from] UtilsError),

    #[error("World error: {0}")]
    World(#[from] WorldError),

    #[error("Inventory error: {0}")]
    Inventory(#[from] InventoryError),

    #[error("{0}")]
    Custom(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Root Path error: {0}")]
    RootPath(#[from] RootPathError),

    #[error("WorldGen error: {0}")]
    WorldGen(#[from] WorldGenError),
}
