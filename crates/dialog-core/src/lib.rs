//! swiftDialog-compatible contract logic: option table, CLI parsing, JSON
//! input/output, exit codes, and the command-file protocol. This crate is
//! deliberately free of any UI/Tauri dependency so the entire scripting
//! contract is testable headless.

pub mod commandfile;
pub mod commands;
pub mod compat;
pub mod config;
pub mod exit_codes;
pub mod options;
pub mod output;
pub mod parser;
pub mod state;
pub mod suboptions;
pub mod validation;
