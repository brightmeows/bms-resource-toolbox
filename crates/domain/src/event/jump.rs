//! BMS event utilities.
//!
//! This module provides utilities for BMS events like BOFTT.

use webbrowser;

/// BMS event types for work information pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum BMSEvent {
    /// BOF Team Festival.
    BOFTT = 20,
    /// BOF 2021.
    BOF21 = 21,
    /// `LetsBMS` Edit 3.
    LetsBMSEdit3 = 103,
}

impl BMSEvent {
    fn from_value(val: i32) -> Self {
        match val {
            21 => BMSEvent::BOF21,
            103 => BMSEvent::LetsBMSEdit3,
            _ => BMSEvent::BOFTT,
        }
    }

    fn list_url(self) -> &'static str {
        match self {
            BMSEvent::BOFTT => "https://manbow.nothing.sh/event/event.cgi?action=sp&event=146",
            BMSEvent::BOF21 => "https://manbow.nothing.sh/event/event.cgi?action=sp&event=149",
            BMSEvent::LetsBMSEdit3 => "https://venue.bmssearch.net/letsbmsedit3",
        }
    }

    fn work_info_url(self, work_num: i32) -> String {
        match self {
            BMSEvent::BOFTT => format!(
                "https://manbow.nothing.sh/event/event.cgi?action=More_def&num={work_num}&event=146"
            ),
            BMSEvent::BOF21 => format!(
                "https://manbow.nothing.sh/event/event.cgi?action=More_def&num={work_num}&event=149"
            ),
            BMSEvent::LetsBMSEdit3 => {
                format!("https://venue.bmssearch.net/letsbmsedit3/{work_num}")
            }
        }
    }
}

/// Jump to work info page for a BMS event.
///
/// Opens URLs for the specified event and work IDs.
/// If `work_ids` is empty, opens the event list page.
pub fn jump_to_work_info(event: BMSEvent, work_ids: &[i32]) {
    if work_ids.is_empty() {
        tracing::info!("Open BMS List.");
        open_url(event.list_url());
        return;
    }

    for &id in work_ids {
        tracing::info!("Open no.{id}");
        open_url(&event.work_info_url(id));
    }
}

impl BMSEvent {
    /// Parse an i32 into a `BMSEvent`, defaulting to BOFTT.
    #[must_use]
    pub fn from_i32(val: i32) -> Self {
        Self::from_value(val)
    }
}

/// Open URL in browser.
pub fn open_url(url: &str) {
    // Intentionally ignored: browser open may fail in headless environments
    let _ = webbrowser::open(url);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bmsevent_work_info_url() {
        let url = BMSEvent::BOFTT.work_info_url(123);
        assert!(url.contains("event=146"));
        assert!(url.contains("num=123"));

        let url = BMSEvent::LetsBMSEdit3.work_info_url(42);
        assert!(url.contains("letsbmsedit3/42"));
    }

    #[test]
    fn test_bmsevent_list_url() {
        assert!(BMSEvent::BOFTT.list_url().contains("event=146"));
        assert!(BMSEvent::BOF21.list_url().contains("event=149"));
        assert!(BMSEvent::LetsBMSEdit3.list_url().contains("letsbmsedit3"));
    }

    #[test]
    fn test_bmsevent_from_i32() {
        assert_eq!(BMSEvent::from_i32(20), BMSEvent::BOFTT);
        assert_eq!(BMSEvent::from_i32(21), BMSEvent::BOF21);
        assert_eq!(BMSEvent::from_i32(103), BMSEvent::LetsBMSEdit3);
        assert_eq!(BMSEvent::from_i32(999), BMSEvent::BOFTT);
    }
}
