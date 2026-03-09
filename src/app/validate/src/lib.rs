use tracing::info;

mod check_chunks;
mod check_players;

pub fn validate() -> Result<(), String> {
    let state = temper_state::create_state(std::time::Instant::now());
    check_chunks::check_chunks(&state)?;
    info!("Chunks validated successfully.");
    check_players::check_players(&state)?;
    info!("Players validated successfully.");

    Ok(())
}
