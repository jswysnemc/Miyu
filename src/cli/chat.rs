use super::*;

pub(super) async fn run_shell_intercept(
    paths: &MiyuPaths,
    shell_name: &str,
    message: String,
    clipb: bool,
) -> Result<()> {
    if !matches!(shell_name, "fish" | "bash" | "zsh" | "powershell") {
        bail!("{}: {shell_name}", t("unsupported shell", "不支持的 shell"));
    }
    if message.is_empty() || !shell::looks_like_natural_language(&message) {
        bail!(
            "{}",
            t("not a natural language command", "不是自然语言命令")
        );
    }
    let result =
        run_chat_with_options(paths, message, None, false, AgentMode::Yolo, clipb, None).await;
    drain_stdin();
    result
}

pub(super) fn drain_stdin() {
    use std::os::fd::AsRawFd;

    let stdin = io::stdin();
    if !stdin.is_terminal() {
        return;
    }
    let fd = stdin.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return;
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return;
    }

    let mut handle = stdin.lock();
    let mut buffer = [0_u8; 4096];
    loop {
        match handle.read(&mut buffer) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }

    let _ = unsafe { libc::fcntl(fd, libc::F_SETFL, flags) };
}

pub(super) async fn run_chat_with_options(
    paths: &MiyuPaths,
    message: String,
    show_reasoning: Option<bool>,
    plain: bool,
    mode: AgentMode,
    clipb: bool,
    thinking_override: Option<String>,
) -> Result<()> {
    if message.is_empty() && !clipb {
        return run_repl(paths, mode, thinking_override).await;
    }
    AppConfig::init_files(paths)?;
    let mut config = AppConfig::load_or_default(paths)?;
    apply_thinking_override(&mut config, thinking_override.as_deref())?;
    let chat_input = prepare_clipboard_chat_input(message, clipb)?;
    let state = StateStore::new(paths)?;
    state.init_files()?;
    let client = OpenAiCompatibleClient::from_config(&config, paths)?;
    let registry = build_tool_registry(&config, paths, mode)?;
    let reasoning_mode = if show_reasoning == Some(false) {
        render::ReasoningDisplayMode::Hidden
    } else {
        render::ReasoningDisplayMode::from_config(&config.display.reasoning)
    };
    let tool_call_mode = if plain {
        render::ToolCallDisplayMode::Hidden
    } else {
        render::ToolCallDisplayMode::from_config(&config.display.tool_calls)
    };
    let render_options = stream_render_options(&config);
    let progressive_loading_enabled = config.tools.progressive_loading_enabled;
    let mut agent = Agent::new(config, paths, state.clone(), client, registry, mode)?;
    if progressive_loading_enabled {
        let loaded_tools = state.load_loaded_tools()?;
        agent.restore_loaded_tools(&loaded_tools);
    }
    let mut renderer =
        render::StreamRenderer::new(reasoning_mode, tool_call_mode, plain, render_options);
    renderer.start_waiting()?;
    let result = agent
        .chat_stream_with_image(&chat_input.message, chat_input.image_url, |event| {
            handle_agent_event(&mut renderer, event)
        })
        .await;
    renderer.finish()?;
    if progressive_loading_enabled {
        state.save_loaded_tools(&agent.loaded_tools())?;
    }
    if let Err(err) = result {
        render::write_chat_error(&err, plain)?;
        return Err(err);
    }
    Ok(())
}
