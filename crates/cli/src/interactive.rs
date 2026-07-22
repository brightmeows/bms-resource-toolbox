//! Interactive menu framework for bms-res-tb.
//!
//! Defines types, traits, and utilities for the interactive menu mode.

pub mod cmd;
pub mod history;
pub mod input;
pub mod menu;
pub mod output;
pub mod path_autocomplete;
pub mod path_validate;
pub mod trait_def;
pub mod types;

/// Shared context for an interactive session.
#[derive(Clone, Copy, Debug)]
pub struct Session {
    /// Whether to skip secondary confirmation prompts.
    pub yes: bool,
}
