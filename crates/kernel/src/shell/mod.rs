//! Shell module - Interactive command-line interface
//!
//! This module provides a modular, extensible shell system with:
//! - I/O abstraction (ShellIO trait)
//! - Command registration and dispatch
//! - Command parsing
//! - Line editing support

pub mod shell;
pub mod commands;

// Re-export commonly used types
pub use shell::{Command, CommandHandler, Shell, ShellIO};
pub use commands::COMMANDS;
