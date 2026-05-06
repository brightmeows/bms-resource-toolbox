use crate::domain::port::OutputPort;

/// Console output adapter — writes to stdout/stderr.
pub struct ConsoleOutput;

impl OutputPort for ConsoleOutput {
    fn info(&self, msg: &str) {
        println!("{msg}");
    }

    fn warn(&self, msg: &str) {
        eprintln!("⚠ {msg}");
    }

    fn error(&self, msg: &str) {
        eprintln!("✗ {msg}");
    }
}
