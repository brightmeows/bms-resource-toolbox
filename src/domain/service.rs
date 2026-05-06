//! Application services — domain logic with injected ports.
//!
//! Each service struct holds port references and provides business operations.
//! Pure computation functions (no I/O) remain free functions.

/// Event service — jump to BMS event work info pages.
pub mod event;
/// Folder cleanup — rename numbered dirs and remove zero-sized media.
pub mod folder_cleanup;
/// Folder media — remove redundant media files by rule.
pub mod folder_media;
/// Folder rename — rename folders by BMS metadata.
pub mod folder_rename;
/// Folder scan — scan and index folder contents.
pub mod folder_scan;
/// Jump — open BMS event pages in browser.
pub mod jump;
/// Pack generate — generate pack archives from folders.
pub mod pack_generate;
/// Pack merge — merge multiple packs into one.
pub mod pack_merge;
/// Pack move — move elements between packs.
pub mod pack_move;
/// Pack split — split packs into individual folders.
pub mod pack_split;
/// Transfer — transfer packs between directories.
pub mod transfer;
/// Unzip — extract archive files.
pub mod unzip;

/// Mock utilities for testing service modules.
#[cfg(test)]
pub mod test_util {
    use std::sync::{Arc, Mutex};

    use crate::domain::port::OutputPort;

    /// Mock output for testing — collects messages for assertion.
    #[derive(Clone, Debug, Default)]
    pub struct MockOutput {
        /// Collected log messages.
        pub messages: Arc<Mutex<Vec<String>>>,
    }

    impl MockOutput {
        /// Create a new empty `MockOutput`.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }
    }

    impl OutputPort for MockOutput {
        fn info(&self, msg: &str) {
            self.messages.lock().unwrap().push(msg.to_string());
        }

        fn warn(&self, msg: &str) {
            self.messages.lock().unwrap().push(format!("WARN: {msg}"));
        }

        fn error(&self, msg: &str) {
            self.messages.lock().unwrap().push(format!("ERR: {msg}"));
        }
    }
}
