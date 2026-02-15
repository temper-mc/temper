use bevy_ecs::prelude::Res;
use std::sync::atomic::Ordering::Relaxed;
use temper_commands::Sender;
use temper_macros::command;
use temper_state::GlobalStateResource;
use tracing::info;

#[command("stop")]
fn stop_command(#[sender] sender: Sender, state: Res<GlobalStateResource>) {
    if !matches!(sender, Sender::Server) {
        sender.send_message("This command can only be used by the server.".into(), false);
        return;
    }
    info!("Shutting down server...");
    state.0.shut_down.store(true, Relaxed);
    state
        .0
        .world
        .sync()
        .expect("Failed to sync world before shutdown")
}
