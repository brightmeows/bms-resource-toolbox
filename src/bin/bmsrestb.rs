//! BMS Resource Toolbox - Entry point.

use bms_resource_toolbox::app::cli;
use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    cli::dispatch(&cli.command);
}
