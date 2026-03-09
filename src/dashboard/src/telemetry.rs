use crate::handshake::Handshake;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate, System};
use temper_config::server_config::get_global_config;
use temper_performance::memory::MemoryUnit;
use temper_state::GlobalState;
use tokio::sync::broadcast::Sender;
use tokio::time::interval;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct ServerMetric {
    /// CPU usage percentage (0.0 - 100.0)
    pub cpu_usage: f32,
    /// Memory usage in bytes
    pub ram_usage: u64,
    /// Total RAM in bytes
    pub total_ram: u64,
    /// Uptime in seconds
    pub uptime: u64,
    /// Used storage in bytes
    pub storage_used: u64,
    /// Number of connected players
    pub player_count: usize,
    pub tps: f32,
    pub players: Vec<PlayerInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerInfo {
    name: String,
    uuid: Uuid,
}

/// Events sent from the server to the dashboard (websocket)
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum DashboardEvent {
    /// One-time handshake sent on connection
    Handshake(Handshake),
    /// Periodic server metrics
    Metric(ServerMetric),
    #[allow(unused)]
    Log(String),
}

pub async fn start_telemetry_loop(tx: Sender<DashboardEvent>, state: GlobalState) {
    debug!("Starting server telemetry");

    // Initialize the system monitor
    let mut sys = System::new_all();
    let pid = Pid::from(std::process::id() as usize);

    // Tick every second; should be configurable later
    const TICK_INTERVAL_SECS: u64 = 1;
    let mut ticker = interval(Duration::from_secs(TICK_INTERVAL_SECS));

    loop {
        ticker.tick().await;

        // Refresh system info for our PID
        sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);

        let Some(process) = sys.process(pid) else {
            error!("Failed to get process info for dashboard telemetry");
            continue;
        };

        let config = get_global_config();
        let mut world_path = temper_general_purpose::paths::get_root_path()
            .join(PathBuf::from(&config.database.db_path));
        let storage_used = if world_path.exists() {
            world_path = world_path.canonicalize().unwrap_or(world_path);
            dir_size::get_size_in_bytes(&world_path).unwrap_or(0)
        } else {
            0
        };

        let cpu_count = sys.cpus().len() as f32;

        let player_count = state.players.player_list.len();
        let mut perf_lock = state
            .performance
            .lock()
            .expect("Failed to lock performance resource");

        let metric = ServerMetric {
            cpu_usage: process.cpu_usage() / cpu_count,
            ram_usage: perf_lock.memory.get_memory(MemoryUnit::Bytes).0,
            total_ram: sys.total_memory(),
            uptime: process.run_time(),
            storage_used,
            player_count,
            tps: perf_lock.tps.tps(Duration::from_secs(1)),
            players: state
                .players
                .player_list
                .iter()
                .map(|kv| PlayerInfo {
                    name: kv.value().1.clone(),
                    uuid: Uuid::from_u128(kv.value().0),
                })
                .collect(),
        };

        // Broadcast to all connected web clients
        // We ignore the error (it fails if no browsers are open, which is fine)
        let _ = tx.send(DashboardEvent::Metric(metric));
    }
}
