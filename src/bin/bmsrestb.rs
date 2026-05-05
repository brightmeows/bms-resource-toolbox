//! BMS Resource Toolbox - Entry point.

// Pre-existing clippy lints — Debug formatting is intentional for logging paths.
#![allow(clippy::unnecessary_debug_formatting)]

use bms_resource_toolbox::app::cli;
use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    cli::dispatch(&cli.command);
}
