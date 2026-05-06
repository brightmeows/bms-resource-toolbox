/// Output port — user feedback abstraction (replaces `println!` / `eprintln!` in domain).
pub trait OutputPort: Send + Sync {
    /// Log an informational message.
    fn info(&self, msg: &str);
    /// Log a warning message.
    fn warn(&self, msg: &str);
    /// Log an error message.
    fn error(&self, msg: &str);
}
