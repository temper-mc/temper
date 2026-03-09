use clap::Parser;
use std::time::Instant;
use temper_app::bin_cli::args::{CLIArgs, Command};
use temper_app::{bin_cli, bin_import, bin_runtime};
use tracing::{error, info};

#[cfg(feature = "dhat")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[cfg(all(feature = "tracy", not(feature = "dhat")))]
#[global_allocator]
static GLOBAL: tracy_client::ProfiledAllocator<std::alloc::System> =
    tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

fn main() {
    #[cfg(feature = "dhat")]
    let _profiler = dhat::Profiler::new_heap();

    let start_time = Instant::now();

    let cli_args = CLIArgs::parse();

    temper_registry::init();

    match cli_args.command {
        Some(Command::Setup) => {
            temper_logging::init_logging(cli_args.log.into(), true);
            info!("Starting setup...");
            if let Err(e) = temper_config::setup::setup() {
                error!("Could not set up the server: {}", e.to_string());
            } else {
                info!("Server setup complete.");
            }
        }

        Some(Command::Import(import_args)) => {
            temper_logging::init_logging(cli_args.log.into(), true);
            info!("Starting import...");
            bin_import::handle_import(import_args);
        }

        Some(Command::Clear(clear_args)) => {
            temper_logging::init_logging(cli_args.log.into(), true);
            if let Err(e) = bin_cli::clear::handle_clear(clear_args) {
                error!("Clear failed: {}", e);
            }
        }

        Some(Command::Run) | None => {
            temper_logging::init_logging(cli_args.log.into(), cli_args.no_tui);
            info!("Starting server...");
            if let Err(e) = temper_config::setup::setup() {
                error!("Could not set up the server: {}", e.to_string());
            } else {
                info!("Server setup complete.");
            }
            if let Err(e) = bin_runtime::entry(start_time, cli_args.no_tui) {
                error!("Server exited with the following error: {}", e.to_string());
            } else {
                info!("Server exited successfully.");
            }
        }

        Some(Command::Validate) => {
            temper_logging::init_logging(cli_args.log.into(), true);
            info!("Starting validation...");
            if let Err(e) = temper_app::bin_validate::validate() {
                error!("Validation failed: {}", e);
            } else {
                info!("Validation complete. No issues found.");
            }
        }
    }
}
