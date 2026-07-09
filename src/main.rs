mod agent;
mod alarm;
mod cli;
mod clipboard;
mod config;
mod config_tui;
mod control_commands;
mod default_kb;
mod default_models;
mod gateways;
mod i18n;
mod llm;
mod memory;
mod paths;
mod perf_trace;
mod prompts;
mod render;
mod runner;
mod runtime_recovery;
mod shell;
mod state;
mod tools;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::parse();
    cli::run(cli).await
}
