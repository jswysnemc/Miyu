mod conversation;
mod message_context;
mod tool_visibility;

use crate::config::AppConfig;
use crate::llm::{
    ChatMessage, ChatResult, ChatStreamChunk, ChatStreamEvent, ChatStreamKind,
    OpenAiCompatibleClient, ToolCallStreamProgress,
};
use crate::memory::MemoryStore;
use crate::paths::MiyuPaths;
use crate::state::{CompactionRequest, PendingTurnGuard, StateStore};
use crate::tools::{self, memes, ToolPermission, ToolRegistry};
use anyhow::{bail, Context, Result};
use message_context::{
    clean_user_visible_text, runtime_context_message, system_messages_first, with_mode_reminder,
};
use tokio::sync::mpsc;
use tool_visibility::ToolVisibility;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AgentMode {
    Yolo,
    Plan,
}

impl AgentMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Yolo => "YOLO",
            Self::Plan => "PLAN",
        }
    }

    fn reminder(self) -> &'static str {
        match self {
            Self::Yolo => crate::prompts::YOLO_REMINDER,
            Self::Plan => crate::prompts::PLAN_REMINDER,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    Chunk(ChatStreamChunk),
    ToolCall {
        name: String,
        arguments: String,
    },
    ToolCallProgress(ToolCallStreamProgress),
    ToolResult {
        name: String,
        ok: bool,
        output: String,
    },
    ToolProgress {
        name: String,
        message: String,
    },
    ExternalOutput,
}

pub struct Agent {
    state: StateStore,
    client: OpenAiCompatibleClient,
    system_prompt: String,
    context_tokens: usize,
    trim_at_ratio: f32,
    tools_enabled: bool,
    max_tool_rounds: usize,
    tools: ToolRegistry,
    tool_visibility: ToolVisibility,
    memory: MemoryStore,
    mode: AgentMode,
    config: AppConfig,
    paths: MiyuPaths,
}

impl Agent {
    pub fn new(
        config: AppConfig,
        paths: &MiyuPaths,
        state: StateStore,
        client: OpenAiCompatibleClient,
        tools: ToolRegistry,
        mode: AgentMode,
    ) -> Result<Self> {
        Self::new_with_extra_system_prompt(config, paths, state, client, tools, mode, None)
    }

    /// 创建带额外系统提示词的 Agent。
    ///
    /// 参数:
    /// - `config`: 应用配置
    /// - `paths`: Miyu 路径
    /// - `state`: 状态存储
    /// - `client`: LLM 客户端
    /// - `tools`: 工具注册表
    /// - `mode`: Agent 模式
    /// - `extra_system_prompt`: 额外系统提示词
    ///
    /// 返回:
    /// - Agent 实例
    pub fn new_with_extra_system_prompt(
        config: AppConfig,
        paths: &MiyuPaths,
        state: StateStore,
        client: OpenAiCompatibleClient,
        mut tools: ToolRegistry,
        mode: AgentMode,
        extra_system_prompt: Option<&str>,
    ) -> Result<Self> {
        let mut base_system_prompt = config.system_prompt(paths)?;
        let tools_enabled = config.tools.enabled && config.active_model_tools_enabled()?;
        if tools_enabled && config.skills.enabled {
            let prompt = if config.tools.progressive_loading_enabled {
                tools::skills_catalog_prompt(&config, paths)?
            } else {
                tools::skills_prompt(&config, paths)?
            };
            if !prompt.trim().is_empty() {
                base_system_prompt.push_str("\n\n");
                base_system_prompt.push_str(&prompt);
            }
        }
        if let Some(prompt) = extra_system_prompt
            .map(str::trim)
            .filter(|prompt| !prompt.is_empty())
        {
            base_system_prompt.push_str("\n\n");
            base_system_prompt.push_str(prompt);
        }
        if mode == AgentMode::Yolo {
            state.reset_if_prompt_changed(&base_system_prompt)?;
            state.recover_stale_turns()?;
        }
        let system_prompt = with_mode_reminder(base_system_prompt, mode);
        let context_tokens = config.active_context_chars()?;
        let max_tool_rounds = config.tools.max_rounds;
        if tools_enabled && config.tools.progressive_loading_enabled {
            tools::register_progressive_loader(&mut tools);
        }
        let tool_visibility = ToolVisibility::new(config.tools.progressive_loading_enabled);
        let memory = MemoryStore::new(&config, paths);
        memory.init()?;
        Ok(Self {
            state,
            client,
            system_prompt,
            context_tokens,
            trim_at_ratio: config.context.trim_at_ratio,
            tools_enabled,
            max_tool_rounds,
            tools,
            tool_visibility,
            memory,
            mode,
            config,
            paths: paths.clone(),
        })
    }

    pub async fn chat_stream<F>(&mut self, input: &str, on_event: F) -> Result<ChatResult>
    where
        F: FnMut(AgentEvent) -> Result<()>,
    {
        self.chat_stream_with_image(input, None, on_event).await
    }

    /// 恢复上一轮已经加载的工具集合。
    ///
    /// 参数:
    /// - `loaded_tools`: 上一轮保存的已加载工具名称
    ///
    /// 返回:
    /// - 无
    pub fn restore_loaded_tools(&mut self, loaded_tools: &[String]) {
        self.tool_visibility
            .restore_loaded_tools(&self.tools, loaded_tools);
    }

    /// 导出当前已经加载的工具集合。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 当前已经加载的工具名称列表
    pub fn loaded_tools(&self) -> Vec<String> {
        self.tool_visibility.loaded_tool_names()
    }

    /// 发送一轮带可选图片的流式对话。
    ///
    /// 参数:
    /// - `input`: 用户文本输入
    /// - `image_url`: 当前轮附加图片 data URL
    /// - `on_event`: 流式事件回调
    ///
    /// 返回:
    /// - 聊天结果
    pub async fn chat_stream_with_image<F>(
        &mut self,
        input: &str,
        image_url: Option<String>,
        on_event: F,
    ) -> Result<ChatResult>
    where
        F: FnMut(AgentEvent) -> Result<()>,
    {
        self.compact_conversation_if_needed().await?;
        let input = clean_user_visible_text(input);
        let turn_id = new_turn_id();
        self.state.start_turn(&turn_id, &input)?;
        let guard = PendingTurnGuard::new(self.state.clone(), turn_id.clone());
        let auto_meme_plan =
            memes::plan_auto_meme_before_reply(&self.config, &self.paths, &self.client, &input)
                .await?;
        let mut messages = self.chat_messages(Some(&turn_id))?;
        if let Some(association) = self.memory.association(&input)? {
            messages.push(ChatMessage::system(
                self.memory.format_association(&association),
            ));
        }
        if let Some(plan) = &auto_meme_plan {
            messages.push(ChatMessage::system(plan.reminder.clone()));
        }
        messages.push(ChatMessage::plain("user", input.clone()));
        if let Some(image_url) = image_url {
            if let Some(message) = messages.last_mut().filter(|message| message.role == "user") {
                *message = ChatMessage::user_with_image(&input, image_url);
            }
        }
        let mut on_event = on_event;
        let mut used_tools = Vec::new();
        let mut persisted_tool_reports = Vec::new();
        let result = self
            .chat_with_tools(
                &mut messages,
                &mut used_tools,
                &mut persisted_tool_reports,
                &mut on_event,
            )
            .await?;
        if let Some(plan) = auto_meme_plan {
            on_event(AgentEvent::ExternalOutput)?;
            memes::render_auto_meme(&self.config, &self.paths, &plan.event).await?;
            memes::record_auto_meme_event(&self.config, &self.paths, &plan.event)?;
        }
        for (tool_name, report) in persisted_tool_reports {
            self.state
                .append_tool_report_context(&turn_id, &tool_name, &report)?;
        }
        guard.complete(&result.content, result.reasoning.as_deref())?;
        self.memory.process_after_turn(&input, &result.content)?;
        if let Some(usage) = &result.usage {
            self.state.add_usage(usage)?;
        }
        Ok(result)
    }

    /// 按最近一次 provider token usage 自动压缩旧会话。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 压缩是否成功
    async fn compact_conversation_if_needed(&self) -> Result<()> {
        let Some(request) = self
            .state
            .select_compaction(self.context_tokens, self.trim_at_ratio)?
        else {
            return Ok(());
        };
        let summary = self
            .create_compaction_summary(&request)
            .await
            .context("failed to compact conversation")?;
        self.state.apply_compaction(&request, &summary)?;
        Ok(())
    }

    /// 立即手动压缩当前会话旧轮次。
    ///
    /// 参数:
    /// - `keep_tail_turns`: 保留的最近非运行轮次数量
    ///
    /// 返回:
    /// - 已压缩轮次数量
    pub async fn compact_conversation_now(&self, keep_tail_turns: usize) -> Result<usize> {
        let Some(request) = self.state.select_manual_compaction(keep_tail_turns)? else {
            return Ok(0);
        };
        let turn_count = request.turn_count();
        let summary = self
            .create_compaction_summary(&request)
            .await
            .context("failed to compact conversation")?;
        self.state.apply_compaction(&request, &summary)?;
        Ok(turn_count)
    }

    /// 使用当前模型生成会话压缩摘要。
    ///
    /// 参数:
    /// - `request`: 压缩请求
    ///
    /// 返回:
    /// - 压缩摘要正文
    async fn create_compaction_summary(&self, request: &CompactionRequest) -> Result<String> {
        let messages = vec![
            ChatMessage::system(
                "You are a conversation compaction worker. Produce a concise, faithful Markdown summary for future turns. Do not answer the user task.",
            ),
            ChatMessage::plain("user", request.prompt()),
        ];
        let result = self
            .client
            .chat_stream_events(messages, Vec::new(), |_| Ok(()))
            .await?;
        let summary = result.content.trim();
        if summary.is_empty() {
            bail!("compaction summary is empty");
        }
        Ok(summary.to_string())
    }

    async fn chat_with_tools<F>(
        &mut self,
        messages: &mut Vec<ChatMessage>,
        used_tools: &mut Vec<String>,
        persisted_tool_reports: &mut Vec<(String, String)>,
        on_event: &mut F,
    ) -> Result<ChatResult>
    where
        F: FnMut(AgentEvent) -> Result<()>,
    {
        let mut tool_round = 0usize;
        loop {
            if self.max_tool_rounds > 0 && tool_round >= self.max_tool_rounds {
                let content = format!(
                    "工具调用已达到上限 {} 轮，已停止继续调用。可将 `tools.max_rounds` 设为 0 以允许无限工具调用。",
                    self.max_tool_rounds
                );
                on_event(AgentEvent::Chunk(ChatStreamChunk {
                    kind: ChatStreamKind::Content,
                    text: content.clone(),
                }))?;
                return Ok(ChatResult {
                    content,
                    reasoning: None,
                    usage: None,
                    tool_calls: Vec::new(),
                });
            }
            tool_round += 1;
            let definitions = if self.tools_enabled {
                self.tool_visibility.definitions(&self.tools)
            } else {
                Vec::new()
            };
            let ordered_messages = system_messages_first(messages.clone());
            let result = self
                .client
                .chat_stream_events(ordered_messages, definitions.clone(), |event| match event {
                    ChatStreamEvent::Chunk(chunk) => on_event(AgentEvent::Chunk(chunk)),
                    ChatStreamEvent::ToolCallProgress(progress) => {
                        on_event(AgentEvent::ToolCallProgress(progress))
                    }
                })
                .await?;
            if result.tool_calls.is_empty() || !self.tools_enabled {
                return Ok(result);
            }
            messages.push(ChatMessage::assistant(
                result.content.clone(),
                Some(result.tool_calls.clone()),
            ));
            for call in result.tool_calls {
                used_tools.push(call.function.name.clone());
                on_event(AgentEvent::ToolCall {
                    name: call.function.name.clone(),
                    arguments: call.function.arguments.clone(),
                })?;
                if self.mode == AgentMode::Plan
                    && self.tools.permission(&call.function.name)? != ToolPermission::ReadOnly
                {
                    bail!(
                        "Plan mode blocked non-read-only tool: {}",
                        call.function.name
                    );
                }
                if self.tool_visibility.is_loader_call(&call.function.name) {
                    let output = match self.tool_visibility.load_from_arguments(
                        &self.tools,
                        &call.function.arguments,
                        &self.config,
                        &self.paths,
                    ) {
                        Ok(output) => {
                            on_event(AgentEvent::ToolResult {
                                name: call.function.name.clone(),
                                ok: true,
                                output: output.clone(),
                            })?;
                            output
                        }
                        Err(err) => {
                            let output = format!("tool error: {err}");
                            on_event(AgentEvent::ToolResult {
                                name: call.function.name.clone(),
                                ok: false,
                                output: output.clone(),
                            })?;
                            output
                        }
                    };
                    messages.push(ChatMessage::tool(
                        call.id,
                        tools::tool_output_for_context(&call.function.name, &output),
                    ));
                    continue;
                }
                if !self.tool_visibility.is_visible(&call.function.name) {
                    let output = format!(
                        "tool error: tool {} is not loaded in the current visible tool set; call load with tool_name or group_name first. If this tool was loaded in a previous conversation, the loaded-tool session state was reset or is unavailable.",
                        call.function.name
                    );
                    on_event(AgentEvent::ToolResult {
                        name: call.function.name.clone(),
                        ok: false,
                        output: output.clone(),
                    })?;
                    messages.push(ChatMessage::tool(
                        call.id,
                        tools::tool_output_for_context(&call.function.name, &output),
                    ));
                    continue;
                }
                if call.function.name == "install_aur_package"
                    && used_tools.iter().any(|name| name == "review_aur_package")
                {
                    let output = "tool error: install_aur_package cannot run in the same turn as review_aur_package. This is a workflow confirmation error, not a tool loading error. Do not call load again; ask the user to confirm installation in a new turn first.".to_string();
                    on_event(AgentEvent::ToolResult {
                        name: call.function.name.clone(),
                        ok: false,
                        output: output.clone(),
                    })?;
                    messages.push(ChatMessage::tool(
                        call.id,
                        tools::tool_output_for_context(&call.function.name, &output),
                    ));
                    continue;
                }
                let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
                let tool_future = self.tools.call_with_progress(
                    &call.function.name,
                    &call.function.arguments,
                    progress_tx,
                );
                tokio::pin!(tool_future);
                let output = loop {
                    tokio::select! {
                        result = &mut tool_future => {
                            break match result {
                                Ok(output) => {
                                    while let Ok(message) = progress_rx.try_recv() {
                                        on_event(AgentEvent::ToolProgress {
                                            name: call.function.name.clone(),
                                            message,
                                        })?;
                                    }
                                    on_event(AgentEvent::ToolResult {
                                        name: call.function.name.clone(),
                                        ok: true,
                                        output: output.clone(),
                                    })?;
                                    if let Some(report) = extract_persistable_tool_report(
                                        &call.function.name,
                                        &output,
                                    ) {
                                        persisted_tool_reports
                                            .push((call.function.name.clone(), report));
                                    }
                                    output
                                }
                                Err(err) => {
                                    while let Ok(message) = progress_rx.try_recv() {
                                        on_event(AgentEvent::ToolProgress {
                                            name: call.function.name.clone(),
                                            message,
                                        })?;
                                    }
                                    on_event(AgentEvent::ToolResult {
                                        name: call.function.name.clone(),
                                        ok: false,
                                        output: format!("tool error: {err}"),
                                    })?;
                                    format!("tool error: {err}")
                                }
                            };
                        }
                        Some(message) = progress_rx.recv() => {
                            on_event(AgentEvent::ToolProgress {
                                name: call.function.name.clone(),
                                message,
                            })?;
                        }
                    }
                };
                messages.push(ChatMessage::tool(
                    call.id,
                    tools::tool_output_for_context(&call.function.name, &output),
                ));
            }
        }
    }

    fn chat_messages(&self, exclude_turn_id: Option<&str>) -> Result<Vec<ChatMessage>> {
        let mut messages = vec![ChatMessage::system(self.system_prompt.clone())];
        if let Some(prompt) = self.tool_visibility.loaded_context_prompt(&self.tools) {
            messages.push(ChatMessage::system(prompt));
        }
        if let Some(summary) = self.state.compaction_summary_context()? {
            messages.push(ChatMessage::system(summary));
        }
        let entries = match exclude_turn_id {
            Some(turn_id) => {
                crate::state::turns_to_entries(self.state.load_turns_excluding(turn_id)?)
            }
            None => self.state.load_conversation()?,
        };
        for entry in entries {
            if entry.role == "user" || entry.role == "assistant" {
                messages.push(ChatMessage::plain(entry.role, entry.content));
            }
        }
        if let Some(summary) = memes::last_auto_meme_reminder(&self.config, &self.paths)? {
            messages.push(ChatMessage::system(summary));
        }
        messages.push(ChatMessage::system(runtime_context_message()));
        Ok(messages)
    }
}

/// 创建当前对话轮次标识。
///
/// 返回:
/// - 当前轮唯一标识
fn new_turn_id() -> String {
    format!(
        "turn_{}_{}",
        chrono::Utc::now().timestamp_millis(),
        rand::random::<u16>()
    )
}

fn extract_persistable_tool_report(tool_name: &str, output: &str) -> Option<String> {
    let field = match tool_name {
        "linux_game_compatibility" => "final_report",
        "linux_input_method_diagnose" | "deep_diagnose" | "deep_research" => "final_answer",
        _ => return None,
    };
    serde_json::from_str::<serde_json::Value>(output)
        .ok()
        .and_then(|value| {
            value
                .get(field)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .map(str::to_string)
        })
        .filter(|report| !report.is_empty())
}
