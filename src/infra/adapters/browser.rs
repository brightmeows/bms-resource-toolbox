use crate::domain::port::BrowserPort;

/// Opens URLs in the system default web browser.
pub struct WebBrowserAdapter;

impl BrowserPort for WebBrowserAdapter {
    fn open(&self, url: &str) {
        let _ = webbrowser::open(url);
    }
}
