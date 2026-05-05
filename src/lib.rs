//! BMS Resource Toolbox Library
//!
//! A Rust library for BMS (Beatmania) chart resource management.

// Pre-existing clippy lint — Debug formatting intentional for logging paths.
#![allow(clippy::unnecessary_debug_formatting)]

pub mod app;
pub mod domain;
pub mod infra;
