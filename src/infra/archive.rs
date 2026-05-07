//! Archive extraction and cache directory flattening.

/// CP437 encoding table for legacy ZIP filename decoding.
pub mod encode;
/// Archive extraction (zip, 7z, rar) and CP932 decoding.
pub mod extract;
