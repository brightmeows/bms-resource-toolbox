/// Port for opening URLs in the system browser.
pub trait BrowserPort: Send + Sync {
    /// Open the given URL in the default browser.
    fn open(&self, url: &str);
}
