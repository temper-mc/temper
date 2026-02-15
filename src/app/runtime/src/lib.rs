mod launch;

use crate::errors::BinaryError;
use std::sync::Arc;
use std::time::Instant;
use temper_config::whitelist::create_whitelist;
use temper_core::pos::ChunkPos;
use temper_state::GlobalState;
use tracing::info;

mod errors;
mod game_loop;
mod tui;

pub fn entry(start_time: Instant, no_tui: bool) -> Result<(), BinaryError> {
    let state = launch::create_state(start_time)?;
    let global_state = Arc::new(state);
    create_whitelist();

    if !global_state
        .world
        .chunk_exists(ChunkPos::new(0, 0), "overworld")?
    {
        launch::generate_spawn_chunks(global_state.clone())?;
    }

    if no_tui {
        ctrlc::set_handler({
            let global_state = global_state.clone();
            move || {
                shutdown_handler(global_state.clone());
            }
        })
        .expect("Error setting Ctrl-C handler");
    }

    #[cfg(feature = "dashboard")]
    temper_dashboard::start_dashboard(global_state.clone());

    game_loop::start_game_loop(global_state.clone(), no_tui)?;

    if !no_tui {
        ratatui::restore()
    }

    Ok(())
}

pub fn shutdown_handler(state: GlobalState) {
    info!("Shutting down server...");
    state
        .shut_down
        .store(true, std::sync::atomic::Ordering::Relaxed);
    state
        .world
        .sync()
        .expect("Failed to sync world before shutdown")
}
