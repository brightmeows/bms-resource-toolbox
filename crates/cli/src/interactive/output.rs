//! User-facing output helpers for the interactive menu.
//!
//! These macros write directly to stdout instead of using tracing,
//! because tracing prefixes every line with timestamps and log levels
//! which is inappropriate for a TUI menu.
//!
//! # Lint compliance
//!
//! Using `std::io::stdout()` + `writeln!` avoids the `disallowed-macros`
//! lint which bans `std::println` / `std::print`.

/// Print a formatted line to stdout (user-facing interactive output).
///
/// Usage: `print_msg!("  {:>2}: {}", i + 1, name);`
macro_rules! print_msg {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(format!($($arg)*).as_bytes());
        let _ = out.write_all(b"\n");
    }};
}

pub(crate) use print_msg;
