mod agent;
mod alarm;
mod cli;
mod clipboard;
mod config;
mod config_tui;
mod control_commands;
mod cron;
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
mod token_estimate;
mod tools;
mod web;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::parse();
    cli::run(cli).await
}
