use crate::agent::{AgentEvent, AgentMode};
use crate::clipboard;
use crate::config::AppConfig;
use crate::gateways::cli::run_gateway;
use crate::i18n::{is_zh, text as t};
use crate::llm::OpenAiCompatibleClient;
use crate::memory::MemoryStore;
use crate::paths::MiyuPaths;
use crate::render;
use crate::shell;
use crate::state::StateStore;
use crate::tools;
use anyhow::{bail, Result};
use crossterm::cursor::{self, Hide, MoveTo, Show};
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyEvent, KeyModifiers,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{execute, queue};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::io::Cursor;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

mod alarm_worker;
mod args;
mod background_commands;
mod chat;
mod config_commands;
mod fuzzy_select;
mod history;
mod init;
mod input_flags;
mod kb_commands;
mod localization;
mod memory_commands;
mod message;
mod model_select;
mod providers;
mod render_options;
mod repl;
mod repl_background;
mod repl_chrome;
mod repl_clipboard;
mod repl_commands;
mod repl_editor;
mod repl_input;
mod repl_input_navigation;
mod repl_input_render;
#[cfg(test)]
mod repl_input_tests;
mod repl_runtime;
mod repl_text;
mod reset;
mod sessions;
mod skills_commands;

use alarm_worker::run_alarm_worker;
use args::*;
use background_commands::run_background_commands;
use chat::{run_chat_with_options, run_shell_intercept, ChatRunOptions};
use config_commands::run_config;
use fuzzy_select::inline_fuzzy_select;
use history::run_history;
use init::{remove_shell_hooks, run_init, InitKind};
use input_flags::parse_message_input_flags;
use kb_commands::{run_kb, run_update_default_kb};
pub(crate) use localization::parse;
use memory_commands::{clear_memory, run_memory};
use message::{join_message, prepare_clipboard_chat_input};
use providers::{apply_thinking_override, run_providers, run_set, run_set_thinking};
use render_options::stream_render_options;
use repl::run_repl;
use repl_background::run_repl_background_manager;
use repl_clipboard::ReplClipboardState;
use repl_commands::{complete_repl_command, repl_command_rest, repl_command_suggestions};
use repl_editor::edit_input_buffer;
use repl_input::read_repl_input;
use repl_input_navigation::{move_cursor_down_by_visual_row, move_cursor_up_by_visual_row};
use repl_input_render::{clear_repl_input, render_repl_input};
use repl_runtime::{process_stream_tick, ReplRuntime};
use repl_text::*;
use reset::run_reset;
use sessions::run_sessions;
use skills_commands::run_skills;

const REPL_MAX_VISIBLE_INPUT_ROWS: u16 = 12;
const REPL_ESC_CLEAR_WINDOW: Duration = Duration::from_millis(650);
const REPL_CTRL_C_EXIT_WINDOW: Duration = Duration::from_millis(900);
const THINKING_LEVELS: &[&str] = &["auto", "none", "low", "medium", "high", "xhigh", "max"];

pub async fn run(cli: Cli) -> Result<()> {
    let paths = MiyuPaths::new()?;
    let thinking_override = cli.thinking.clone();
    let mode = if cli.plan {
        AgentMode::Plan
    } else {
        AgentMode::Yolo
    };

    if cli.shell_intercept {
        let shell_name = cli.shell.as_deref().unwrap_or("fish");
        let input = parse_message_input_flags(cli.message, cli.clipb, cli.web_search);
        return run_shell_intercept(
            &paths,
            shell_name,
            input.message,
            input.clipb,
            input.web_search,
        )
        .await;
    }

    if !paths.config_file.exists() && !matches!(cli.command, Some(Command::Init)) {
        run_init(&paths, InitKind::FirstRun)?;
    }

    match cli.command {
        Some(Command::AlarmWorker(args)) => run_alarm_worker(args),
        Some(Command::Tool(args)) => run_tool(&paths, mode, args).await,
        Some(Command::Ask(args)) => {
            let input = parse_message_input_flags(args.message, args.clipb, args.web_search);
            run_chat_with_options(
                &paths,
                ChatRunOptions {
                    message: input.message,
                    source: crate::runner::SubmissionSource::Command,
                    show_reasoning: None,
                    plain: false,
                    mode,
                    clipb: input.clipb,
                    web_search: input.web_search,
                    thinking_override: args.thinking.or_else(|| thinking_override.clone()),
                    show_final_summary: true,
                },
            )
            .await
        }
        Some(Command::Init) => run_init(&paths, InitKind::Explicit),
        Some(Command::Paths) => {
            paths.print();
            Ok(())
        }
        Some(Command::Config(args)) => run_config(&paths, args).await,
        Some(Command::Providers(args)) => run_providers(&paths, args),
        Some(Command::FishInit) => shell::fish::install(&paths),
        Some(Command::BashInit) => shell::bash::install(&paths),
        Some(Command::ZshInit) => shell::zsh::install(&paths),
        Some(Command::PowershellInit) => shell::powershell::install(&paths),
        Some(Command::RemoveShellHook) => remove_shell_hooks(&paths),
        Some(Command::History(args)) => run_history(&paths, args),
        Some(Command::Sessions(args)) => run_sessions(&paths, args),
        Some(Command::Resume(args)) => sessions::run_resume(&paths, args),
        Some(Command::Kb(args)) => run_kb(&paths, args).await,
        Some(Command::UpdateDefaultKb) => run_update_default_kb(&paths).await,
        Some(Command::Memory(args)) => run_memory(&paths, args),
        Some(Command::Skills(args)) => run_skills(&paths, args),
        Some(Command::Ps(args)) => run_background_commands(&paths, args).await,
        Some(Command::Gateway(args)) => run_gateway(&paths, args).await,
        Some(Command::Set(args)) => run_set(&paths, args),
        Some(Command::Clear(args)) => run_reset(&paths, args.scope.as_deref(), args.memory),
        Some(Command::Compact(args)) => {
            let keep_tail_turns = args
                .keep_tail_turns
                .unwrap_or(crate::control_commands::DEFAULT_KEEP_TAIL_TURNS);
            println!(
                "{}",
                crate::control_commands::compact_conversation_from_paths(&paths, keep_tail_turns)
                    .await?
            );
            Ok(())
        }
        None => {
            let input = parse_message_input_flags(cli.message, cli.clipb, cli.web_search);
            if input.message.is_empty() && !input.clipb && !input.web_search {
                run_repl(&paths, mode, thinking_override.clone()).await
            } else {
                run_chat_with_options(
                    &paths,
                    ChatRunOptions {
                        message: input.message,
                        source: crate::runner::SubmissionSource::Command,
                        show_reasoning: None,
                        plain: false,
                        mode,
                        clipb: input.clipb,
                        web_search: input.web_search,
                        thinking_override: thinking_override.clone(),
                        show_final_summary: true,
                    },
                )
                .await
            }
        }
    }
}

async fn run_tool(paths: &MiyuPaths, mode: AgentMode, args: ToolArgs) -> Result<()> {
    let config = AppConfig::load_or_default(paths)?;
    let registry = build_tool_registry(&config, paths, mode)?;
    let output = registry
        .call(&args.name, args.arguments.as_deref().unwrap_or("{}"))
        .await?;
    println!("{output}");
    Ok(())
}

/// 构建通用工具注册表。
///
/// 参数:
/// - `config`: 应用配置
/// - `paths`: Miyu 路径
/// - `mode`: 当前 Agent 模式
///
/// 返回:
/// - 工具注册表
pub(crate) fn build_tool_registry(
    config: &AppConfig,
    paths: &MiyuPaths,
    mode: AgentMode,
) -> Result<tools::ToolRegistry> {
    let mut registry = if config.tools.enabled {
        match mode {
            AgentMode::Yolo => tools::builtin_registry(config, paths),
            AgentMode::Plan => tools::readonly_registry(config, paths),
        }
    } else {
        tools::ToolRegistry::new()
    };
    if mode == AgentMode::Yolo && config.tools.enabled && config.skills.enabled {
        tools::register_skills(&mut registry, config, paths, true)?;
    }
    Ok(registry)
}

pub(crate) fn build_repl_tool_registry(
    config: &AppConfig,
    paths: &MiyuPaths,
    mode: AgentMode,
) -> Result<tools::ToolRegistry> {
    let mut registry = build_tool_registry(config, paths, mode)?;
    if mode == AgentMode::Yolo && config.tools.enabled {
        tools::register_repl_task_tool(&mut registry, config, paths);
    }
    Ok(registry)
}

fn handle_agent_event(renderer: &mut render::StreamRenderer, event: AgentEvent) -> Result<()> {
    match event {
        AgentEvent::Chunk(chunk) => renderer.write_chunk(chunk),
        AgentEvent::ToolCall { name, arguments } => renderer.write_tool_call(&name, &arguments),
        AgentEvent::ToolCallProgress(progress) => renderer.write_tool_call_progress(&progress),
        AgentEvent::ToolResult { name, ok, output } => {
            renderer.write_tool_result(&name, ok, &output)
        }
        AgentEvent::ToolProgress { name, message } => renderer.write_tool_progress(&name, &message),
        AgentEvent::CompactionStarted { turn_count } => {
            renderer.write_compaction_started(turn_count)
        }
        AgentEvent::CompactionFinished { applied } => renderer.write_compaction_finished(applied),
        AgentEvent::FlushContent => renderer.flush_content(),
        AgentEvent::ExternalOutput => renderer.prepare_for_external_output(),
    }
}
