use super::*;

pub(super) async fn run_repl(
    paths: &MiyuPaths,
    initial_mode: AgentMode,
    thinking_override: Option<String>,
) -> Result<()> {
    AppConfig::init_files(paths)?;
    let mut config = AppConfig::load_or_default(paths)?;
    apply_thinking_override(&mut config, thinking_override.as_deref())?;
    let state = StateStore::new(paths)?;
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
    println!("{}", t("commands:", "命令:"));
    println!(
        "  /providers  {}",
        t("switch provider or model", "切换 provider 或模型")
    );
    println!(
        "  /config     {}",
        t("open configuration UI", "打开配置界面")
    );
    println!(
        "  /thinking [level] {}",
        t(
            "choose active provider thinking level",
            "选择当前 provider 的思考等级"
        )
    );
    println!(
        "  /ps        {}",
        t("manage background tasks", "管理后台任务")
    );
    println!(
        "  /plan       {}",
        t("switch to read-only planning mode", "切换到只读计划模式")
    );
    println!(
        "  /yolo       {}",
        t("switch to YOLO mode", "切换到 YOLO 模式")
    );
    println!(
        "  /undo       {}",
        t(
            "remove last turn and restore prompt",
            "撤销上一轮并恢复输入"
        )
    );
    println!(
        "  /reset [all] {}",
        t(
            "clear current conversation history; all also clears memory",
            "清空当前会话历史；all 同时清空记忆"
        )
    );
    println!("  /help       {}", t("show this help", "显示此帮助"));
    println!("  /exit       {}", t("leave REPL", "退出 REPL"));
    println!("{}", t("keys:", "快捷键:"));
    println!(
        "  Tab         {}",
        t(
            "toggle YOLO/PLAN, or complete slash commands",
            "切换 YOLO/PLAN，或补全斜杠菜单"
        )
    );
    println!("  Enter       {}", t("send message", "发送消息"));
    println!("  Shift+Enter {}", t("insert newline", "插入换行"));
    println!(
        "  Ctrl+J      {}",
        t(
            "insert newline, same as Shift+Enter",
            "插入换行，与 Shift+Enter 相同"
        )
    );
    println!(
        "  Ctrl+V      {}",
        t("paste clipboard text or image", "粘贴剪贴板文本或图片")
    );
    println!("  Ctrl+L      {}", t("clear screen", "清屏"));
    println!(
        "  Ctrl+G      {}",
        t(
            "edit input buffer in $EDITOR",
            "使用 $EDITOR 编辑输入缓冲区"
        )
    );
    println!(
        "  Up/Down     {}",
        t("browse input history", "切换输入历史")
    );
    println!(
        "  Esc Esc     {}",
        t("clear current message", "清空当前消息")
    );
    println!("  Ctrl+C Ctrl+C {}", t("exit REPL", "退出 REPL"));
}
