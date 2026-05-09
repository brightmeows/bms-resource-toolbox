//! Domain operations — business logic that orchestrates infra and bms.

pub mod bms;
pub mod error;
pub mod event;
pub mod folder;
pub mod pack;
/// Selective directory synchronization with comparison presets.
pub mod sync;
pub mod transfer;
pub mod util;
