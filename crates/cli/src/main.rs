//! BMS Resource Toolbox - Entry point.

mod cli;
mod dispatch;

use clap::Parser;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let cli = cli::Cli::parse();
    if let Err(e) = dispatch::dispatch(&cli.command, cli.yes).await {
        tracing::error!("{e:#}");
    }
}
