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
                    crate::control_commands::ControlCommand::Resume { id } => {
                        let session_id = match id {
                            Some(id) => id,
                            None => match sessions::select_session_id_interactively(paths) {
                                Ok(id) => id,
                                Err(err) => {
                                    eprintln!("{err}");
                                    continue;
                                }
                            },
                        };
                        match crate::control_commands::resume_session(paths, &session_id) {
                            Ok(message) => {
                                println!("{message}");
                                state = StateStore::new(paths)?;
                                state.init_files()?;
                                input_history = load_repl_input_history(&state)?;
                                prefill = None;
                            }
                            Err(err) => eprintln!("{err}"),
                        }
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
        if input.eq_ignore_ascii_case("/clear") {
            run_reset(paths, None)?;
            input_history.clear();
            continue;
        }
        if input.eq_ignore_ascii_case("/clear all") {
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
        let reasoning_mode = render::ReasoningDisplayMode::from_config(&config.display.reasoning);
        let tool_call_mode = render::ToolCallDisplayMode::from_config(&config.display.tool_calls);
        let render_options = stream_render_options(&config);
        let runner_submission = repl_runner_submission(
            chat_input,
            mode,
            reasoning_mode,
            tool_call_mode,
            render_options.clone(),
        );
        let mut renderer =
            render::StreamRenderer::new(reasoning_mode, tool_call_mode, false, render_options);
        renderer.start_waiting()?;
        let mut interrupted = false;
        let mut runner_output = crate::runner::RunnerOutput::default();
        let chat_result = {
            let runner = crate::runner::SessionRunner::new(paths).with_config(config.clone());
            let mut sink = |event: crate::runner::RunnerEvent| {
                handle_repl_runner_event(&mut renderer, &event)?;
                runner_output.push_event(event);
                Ok(())
            };
            let chat = runner.run_submission(runner_submission, &mut sink);
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
        if interrupted {
            runner_output.push_event(crate::runner::RunnerEvent::Interrupted);
        }
        renderer.finish()?;
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

/// 构造 REPL 单轮 runner submission。
///
/// 参数:
/// - `chat_input`: 剪贴板处理后的聊天输入
/// - `mode`: 当前 Agent 模式
/// - `reasoning_mode`: 推理内容显示方式
/// - `tool_call_mode`: 工具调用显示方式
/// - `render_options`: 流式渲染选项
///
/// 返回:
/// - runner submission
fn repl_runner_submission(
    chat_input: clipboard::ClipboardChatInput,
    mode: AgentMode,
    reasoning_mode: render::ReasoningDisplayMode,
    tool_call_mode: render::ToolCallDisplayMode,
    render_options: render::StreamRenderOptions,
) -> crate::runner::RunnerSubmission {
    let user_input = match chat_input.image_url {
        Some(image_url) => crate::runner::UserInputSubmission::new(chat_input.message, mode)
            .with_image_url(image_url),
        None => crate::runner::UserInputSubmission::new(chat_input.message, mode),
    };
    crate::runner::RunnerSubmission::user_input(crate::runner::SubmissionSource::Repl, user_input)
        .with_render_policy(crate::runner::RenderPolicy::new(
            false,
            reasoning_mode,
            tool_call_mode,
            render_options,
        ))
}

/// 将 runner 事件投影到 REPL renderer。
///
/// 参数:
/// - `renderer`: 当前流式 renderer
/// - `event`: runner 事件
///
/// 返回:
/// - 渲染是否成功
fn handle_repl_runner_event(
    renderer: &mut render::StreamRenderer,
    event: &crate::runner::RunnerEvent,
) -> Result<()> {
    if let crate::runner::RunnerEvent::Agent(agent_event) = event {
        handle_agent_event(renderer, agent_event.clone())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证 REPL 聊天输入会构造成 runner submission。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn repl_chat_input_builds_runner_submission() {
        let submission = repl_runner_submission(
            clipboard::ClipboardChatInput {
                message: "继续".to_string(),
                image_url: Some("data:image/png;base64,AAAA".to_string()),
            },
            AgentMode::Yolo,
            render::ReasoningDisplayMode::Summary,
            render::ToolCallDisplayMode::Summary,
            render::StreamRenderOptions::default(),
        );

        assert_eq!(submission.source, crate::runner::SubmissionSource::Repl);
        assert!(matches!(
            submission.kind,
            crate::runner::RunnerSubmissionKind::UserInput(crate::runner::UserInputSubmission {
                image_url: Some(_),
                ..
            })
        ));
    }
}
