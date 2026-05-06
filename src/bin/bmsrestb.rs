//! BMS Resource Toolbox - Entry point.

use bms_resource_toolbox::app::{bootstrap, cli};
use clap::Parser;

#[tokio::main]
async fn main() {
    let ctx = bootstrap::bootstrap();
    let cli = cli::Cli::parse();
    cli::dispatch(&ctx, &cli.command);
}
