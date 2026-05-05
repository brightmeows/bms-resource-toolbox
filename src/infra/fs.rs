//! File system utilities.

/// Filename sanitization and BMS directory similarity comparison.
pub mod name;
/// File move, merge, and replacement logic.
pub mod pack_move;
/// Soft sync (selective directory synchronization with comparison presets).
pub mod sync;
/// Miscellaneous file system utilities (extension extraction, recursive copy).
pub mod utils;
/// Empty directory removal.
pub mod walk;
