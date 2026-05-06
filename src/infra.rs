//! Infrastructure layer — stateless utilities with no business logic.

/// Hexagonal architecture adapters (port implementations).
pub mod adapters;
/// Archive extraction and cache directory flattening.
pub mod archive;
pub mod fs;
pub mod media;
