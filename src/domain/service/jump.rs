use crate::domain::port::BrowserPort;

/// BMS event types for work information pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    /// Parse an i32 into a `BMSEvent`, defaulting to BOFTT.
    #[must_use]
    pub fn from_i32(val: i32) -> Self {
        Self::from_value(val)
    }
}

/// Service for opening BMS chart URLs in the system browser.
pub struct JumpService {
    browser: Box<dyn BrowserPort>,
}

impl JumpService {
    /// Create a new `JumpService` with the given browser port.
    #[must_use]
    pub fn new(browser: Box<dyn BrowserPort>) -> Self {
        Self { browser }
    }

    /// Jump to work info pages for the given event and work IDs.
    /// Opens the event list page if `work_ids` is empty.
    pub fn jump_to_work_info(&self, event: BMSEvent, work_ids: &[i32]) {
        if work_ids.is_empty() {
            self.browser.open(event.list_url());
            return;
        }

        for &id in work_ids {
            self.browser.open(&event.work_info_url(id));
        }
    }
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
