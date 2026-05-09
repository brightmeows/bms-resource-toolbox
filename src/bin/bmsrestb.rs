//! BMS Resource Toolbox - Entry point.

use bms_resource_toolbox::app::cli;
use clap::Parser;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let cli = cli::Cli::parse();
    if let Err(e) = cli::dispatch(&cli.command, cli.yes).await {
        tracing::error!("{e:#}");
    }
}
