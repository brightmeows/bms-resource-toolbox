//! BMS Resource Toolbox - Entry point.

mod cli;
mod dispatch;
mod interactive;

use clap::Parser;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let cli = cli::Cli::parse();

    match cli.command {
        Some(cmd) => {
            // CLI mode: dispatch to domain functions
            if let Err(e) = dispatch::dispatch(&cmd, cli.yes).await {
                tracing::error!("{e:#}");
            }
        }
        None => {
            // Interactive menu mode
            if let Err(e) = interactive::menu::run_main_menu(cli.yes).await {
                tracing::error!("{e:#}");
            }
        }
    }
}
