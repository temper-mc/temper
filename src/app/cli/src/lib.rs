//! Command-line interface module for temper.
//!
//! This module provides all CLI argument parsing and command handling
//! functionality for the temper Minecraft server.
//!
//! # Submodules
//!
//! - [`clear`] - Clear command for removing server data
//!
//! # Example
//!
//! ```bash
//! # Run the server
//! temper run
//!
//! # Clear all server data
//! temper clear --all
//! ```

pub mod args;
pub mod clear;
pub mod errors;
