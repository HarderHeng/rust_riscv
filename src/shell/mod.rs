//! Shell subsystem for bare-metal RISC-V kernel
//!
//! This module provides a modular, extensible command-line shell with:
//! - I/O abstraction (supports different backends)
//! - Command registration and dispatch
//! - Line editing (backspace, Ctrl+C, Ctrl+L)
//! - Built-in commands
//!
//! # Usage
//!
//! ```rust,no_run
//! use shell::{Shell, UartIO};
//! use shell::commands::COMMANDS;
//!
//! let io = UartIO::uart0();
//! io.init();
//!
//! let mut shell = Shell::new(io, COMMANDS, "kernel> ");
//! shell.run(); // Never returns
//! ```

pub mod shell;
pub mod uart_io;
pub mod commands;

// Re-export commonly used types
pub use shell::Shell;
pub use uart_io::UartIO;