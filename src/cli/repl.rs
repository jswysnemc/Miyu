use super::*;

pub(super) async fn run_repl(
    paths: &MiyuPaths,
    initial_mode: AgentMode,
    thinking_override: Option<String>,
) -> Result<()> {
    AppConfig::init_files(paths)?;
    let mut config = AppConfig::load_or_default(paths)?;
    apply_thinking_override(&mut config, thinking_override.as_deref())?;
    let mut state = StateStore::new(paths)?;
    state.init_files()?;
    let mut client = OpenAiCompatibleClient::from_config(&config, paths)?;
    let mut mode = initial_mode;
    let mut input_history = load_repl_input_history(&state)?;
    let mut prefill = None::<String>;

    println!(
        "\x1b[2m{}\x1b[0m",
        t(
            "Tab toggles mode; Enter sends; Shift+Enter/Ctrl+J inserts newline; Ctrl+V pastes clipboard",
            "Tab 切换模式；Enter 发送；Shift+Enter/Ctrl+J 换行；Ctrl+V 粘贴剪贴板",
        )
    );
    crate::default_kb::check_update_if_due(paths).ok();
    if let Ok(Some(message)) = crate::default_kb::notice_if_update_available(paths) {
        println!("\x1b[2m{message}\x1b[0m");
    }
    loop {
        let submission = match read_repl_input(mode, prefill.take(), &input_history)? {
            Some(submission) => {
                mode = submission.mode;
                submission
            }
            None => break,
        };
        let input = submission.raw_input.trim();
        let submitted_input = input.to_string();
        if input.eq_ignore_ascii_case("exit")
            || input.eq_ignore_ascii_case("quit")
            || input.eq_ignore_ascii_case("/exit")
        {
            break;
        }
        match crate::control_commands::parse_control_command(
            input,
            crate::control_commands::ControlSurface::Repl,
        ) {
            Ok(Some(command)) => {
                match command {
                    crate::control_commands::ControlCommand::Help => print_repl_help(),
                    crate::control_commands::ControlCommand::New { title } => {
                        println!(
                            "{}",
                            crate::control_commands::create_new_session(paths, &title)?
                        );
                        state = StateStore::new(paths)?;
                        state.init_files()?;
                        input_history = load_repl_input_history(&state)?;
                        prefill = None;
                    }
                    crate::control_commands::ControlCommand::Compact { keep_tail_turns } => {
                        let message = crate::control_commands::compact_conversation_with_agent(
                            &config,
                            paths,
                            &state,
                            &client,
                            mode,
                            keep_tail_turns,
                        )
                        .await?;
                        println!("{message}");
                        input_history = load_repl_input_history(&state)?;
                    }
                    crate::control_commands::ControlCommand::Clear { all } => {
                        println!("{}", crate::control_commands::clear_state(paths, all)?);
                        input_history.clear();
                    }
                    crate::control_commands::ControlCommand::Model { selection } => {
                        let result = crate::control_commands::run_model_command(
                            paths,
                            selection,
                            crate::control_commands::ControlSurface::Repl,
                        )?;
                        println!("{}", result.message);
                        if result.changed {
                            reload_repl_config(
                                paths,
                                &mut config,
                                &mut client,
                                thinking_override.as_deref(),
                            )?;
                            println!("{}", t("configuration reloaded", "配置已重新加载"));
                        }
                    }
                }
                continue;
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("{err}");
                continue;
            }
        }
        if input.eq_ignore_ascii_case("/help") {
            print_repl_help();
            continue;
        }
        if input.eq_ignore_ascii_case("/plan") {
            mode = AgentMode::Plan;
            println!("{}: {}", t("mode", "模式"), mode.label());
            continue;
        }
        if input.eq_ignore_ascii_case("/yolo") {
            mode = AgentMode::Yolo;
            println!("{}: {}", t("mode", "模式"), mode.label());
            continue;
        }
        if input.eq_ignore_ascii_case("/providers") {
            run_providers(paths, ProvidersArgs { index: None })?;
            reload_repl_config(
                paths,
                &mut config,
                &mut client,
                thinking_override.as_deref(),
            )?;
            println!("{}", t("configuration reloaded", "配置已重新加载"));
            continue;
        }
        if input.eq_ignore_ascii_case("/config") {
            crate::config_tui::run(paths)?;
            reload_repl_config(
                paths,
                &mut config,
                &mut client,
                thinking_override.as_deref(),
            )?;
            println!("{}", t("configuration reloaded", "配置已重新加载"));
            continue;
        }
        if input.eq_ignore_ascii_case("/undo") {
            let (removed, prompt) = state.undo_last_turn()?;
            println!("{}: {removed}", t("undone messages", "已撤销消息数"));
            prefill = prompt;
            continue;
        }
        if input.eq_ignore_ascii_case("/reset") {
            run_reset(paths, None)?;
            input_history.clear();
            continue;
        }
        if input.eq_ignore_ascii_case("/reset all") {
            run_reset(paths, Some("all"))?;
            input_history.clear();
            continue;
        }
        if let Some(rest) = repl_command_rest(input, "/thinking") {
            let level = rest
                .split_whitespace()
                .next()
                .map(std::string::ToString::to_string);
            if let Err(err) = run_set_thinking(paths, SetThinkingArgs { level }) {
                eprintln!("{err}");
                continue;
            }
            reload_repl_config(
                paths,
                &mut config,
                &mut client,
                thinking_override.as_deref(),
            )?;
            println!("{}", t("configuration reloaded", "配置已重新加载"));
            continue;
        }
        if input.eq_ignore_ascii_case("/ps") {
            run_repl_background_manager(paths, &config).await?;
            continue;
        }
        if input.is_empty() {
            continue;
        }
        let chat_input = submission.chat_input;
        if chat_input.message.trim().is_empty() && chat_input.image_url.is_none() {
            continue;
        }
        if !input.trim().is_empty() {
            input_history.push(input.to_string());
        }
        render_repl_submitted_input(mode, input)?;
        let registry = build_repl_tool_registry(&config, paths, mode)?;
        let mut agent = Agent::new(
            config.clone(),
            paths,
            state.clone(),
            client.clone(),
            registry,
            mode,
        )?;
        if config.tools.progressive_loading_enabled {
            let loaded_tools = state.load_loaded_tools()?;
            agent.restore_loaded_tools(&loaded_tools);
        }
        let reasoning_mode = render::ReasoningDisplayMode::from_config(&config.display.reasoning);
        let tool_call_mode = render::ToolCallDisplayMode::from_config(&config.display.tool_calls);
        let mut renderer = render::StreamRenderer::new(
            reasoning_mode,
            tool_call_mode,
            false,
            stream_render_options(&config),
        );
        renderer.start_waiting()?;
        let mut interrupted = false;
        let chat_result = if let Some(image_url) = chat_input.image_url {
            let chat =
                agent.chat_stream_with_image(&chat_input.message, Some(image_url), |event| {
                    handle_agent_event(&mut renderer, event)
                });
            tokio::pin!(chat);
            tokio::select! {
                result = &mut chat => result.map(|_| ()),
                signal = tokio::signal::ctrl_c() => {
                    signal?;
                    interrupted = true;
                    Ok(())
                }
            }
        } else {
            let chat = agent.chat_stream(&chat_input.message, |event| {
                handle_agent_event(&mut renderer, event)
            });
            tokio::pin!(chat);
            tokio::select! {
                result = &mut chat => result.map(|_| ()),
                signal = tokio::signal::ctrl_c() => {
                    signal?;
                    interrupted = true;
                    Ok(())
                }
            }
        };
        renderer.finish()?;
        if config.tools.progressive_loading_enabled {
            state.save_loaded_tools(&agent.loaded_tools())?;
        }
        if interrupted {
            prefill = Some(submitted_input);
            continue;
        }
        if let Err(err) = chat_result {
            render::write_chat_error(&err, false)?;
            continue;
        }
    }
    Ok(())
}

/// 渲染 REPL 中已提交的输入行。
///
/// 参数:
/// - `mode`: 当前 Agent 模式
/// - `input`: 用户提交的输入内容
///
/// 返回:
/// - 渲染是否成功
fn render_repl_submitted_input(mode: AgentMode, input: &str) -> Result<()> {
    let lines = repl_input_lines(input);
    let prompt_prefix = format!("{} > ", colored_mode_label(mode));
    for (index, line) in lines.iter().enumerate() {
        if index == 0 {
            println!("{prompt_prefix}{line}");
        } else {
            println!("{line}");
        }
    }
    Ok(())
}

fn reload_repl_config(
    paths: &MiyuPaths,
    config: &mut AppConfig,
    client: &mut OpenAiCompatibleClient,
    thinking_override: Option<&str>,
) -> Result<()> {
    *config = AppConfig::load(paths)?;
    apply_thinking_override(config, thinking_override)?;
    *client = OpenAiCompatibleClient::from_config(config, paths)?;
    Ok(())
}

pub(super) fn load_repl_input_history(state: &StateStore) -> Result<Vec<String>> {
    Ok(state
        .load_conversation()?
        .into_iter()
        .filter(|entry| entry.role == "user" && !entry.content.trim().is_empty())
        .map(|entry| strip_terminal_control_sequences(&entry.content))
        .filter(|content| !content.trim().is_empty())
        .collect())
}

fn print_repl_help() {
    println!(
        "{}",
        crate::control_commands::help_text(crate::control_commands::ControlSurface::Repl)
    );
}
