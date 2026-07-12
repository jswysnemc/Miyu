use super::tool_call_stream::ToolCallProgressTracker;
use super::{
    ChatMessage, ChatResult, ChatStreamChunk, ChatStreamEvent, ChatStreamKind, ToolCall,
    ToolCallFunction, ToolCallStreamProgress, ToolDefinition, Usage,
};
use crate::config::{AppConfig, ProviderConfig};
use crate::i18n::text as t;
use crate::llm::http_debug::{
    anthropic_request_headers, bearer_request_headers, HttpDebugConfig, HttpDebugRecorder,
};
use crate::llm::thinking::{apply_provider_body_options, ThinkingProtocol};
use crate::paths::MiyuPaths;
use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderProtocol {
    Auto,
    OpenAiChat,
    OpenAiResponses,
    Anthropic,
}

impl ProviderProtocol {
    fn from_provider(provider: &ProviderConfig) -> Result<Self> {
        match provider.protocol.trim() {
            "" | "auto" => Ok(Self::Auto),
            "openai-chat" => Ok(Self::OpenAiChat),
            "openai-responses" => Ok(Self::OpenAiResponses),
            "anthropic" | "anthropic-messages" | "messages" | "claude" | "claude-code" => {
                Ok(Self::Anthropic)
            }
            protocol => bail!("unsupported provider protocol: {protocol}"),
        }
    }
}

#[derive(Clone)]
pub struct OpenAiCompatibleClient {
    client: Client,
    provider: ProviderConfig,
    api_key: String,
    /// 可选 HTTP 调试落盘配置（`MIYU_DEBUG_HTTP`）
    http_debug: Option<HttpDebugConfig>,
}

impl OpenAiCompatibleClient {
    pub fn from_config(config: &AppConfig, paths: &MiyuPaths) -> Result<Self> {
        let provider = config.provider(None)?;
        Self::new(provider, config, paths)
    }

    pub fn new(provider: &ProviderConfig, _config: &AppConfig, paths: &MiyuPaths) -> Result<Self> {
        if provider.default_model.trim().is_empty() {
            bail!(
                "{}: {}",
                t(
                    "provider has no active model; select a model before chatting",
                    "provider 没有当前模型；请先选择模型再聊天",
                ),
                provider.id
            );
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(provider.timeout_seconds.clamp(5, 30)))
            .build()?;
        let api_key = provider.resolved_api_key(paths)?;
        Ok(Self {
            client,
            provider: provider.clone(),
            api_key,
            http_debug: HttpDebugConfig::from_env(paths),
        })
    }

    /// 在调试开启时开始一次请求记录。
    ///
    /// 参数:
    /// - `method`: HTTP 方法
    /// - `url`: 请求 URL
    /// - `protocol`: 协议标签
    /// - `headers`: 请求头
    /// - `body`: 请求体
    ///
    /// 返回:
    /// - 可选记录器
    fn start_http_debug(
        &self,
        method: &str,
        url: &str,
        protocol: &str,
        headers: &[(String, String)],
        body: &Value,
    ) -> Option<HttpDebugRecorder> {
        let config = self.http_debug.as_ref()?;
        match HttpDebugRecorder::start(
            config,
            method,
            url,
            &self.provider.id,
            protocol,
            headers,
            body,
        ) {
            Ok(recorder) => recorder,
            Err(err) => {
                eprintln!("[miyu] HTTP debug start failed: {err:#}");
                None
            }
        }
    }

    pub async fn chat_stream<F>(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        mut on_chunk: F,
    ) -> Result<ChatResult>
    where
        F: FnMut(ChatStreamChunk) -> Result<()>,
    {
        self.chat_stream_events(messages, tools, |event| {
            if let ChatStreamEvent::Chunk(chunk) = event {
                on_chunk(chunk)?;
            }
            Ok(())
        })
        .await
    }

    /// 发送流式对话并透出内部流式事件。
    ///
    /// 参数:
    /// - `messages`: 聊天消息列表
    /// - `tools`: 当前可用工具定义
    /// - `on_event`: 流式事件回调
    ///
    /// 返回:
    /// - 聊天结果
    pub async fn chat_stream_events<F>(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        mut on_event: F,
    ) -> Result<ChatResult>
    where
        F: FnMut(ChatStreamEvent) -> Result<()>,
    {
        let protocol = ProviderProtocol::from_provider(&self.provider)?;
        if protocol == ProviderProtocol::Anthropic {
            return self
                .chat_anthropic_stream(messages, tools, &mut on_event)
                .await;
        }
        if protocol == ProviderProtocol::OpenAiResponses
            || (protocol == ProviderProtocol::Auto && self.uses_openai_responses())
        {
            if let Some(result) = self
                .chat_responses_stream(messages.clone(), tools.clone(), &mut on_event)
                .await?
            {
                return Ok(result);
            }
            if protocol == ProviderProtocol::OpenAiResponses {
                bail!("OpenAI Responses protocol is not supported by this provider");
            }
        }
        let request = ChatRequest {
            model: self.provider.default_model.clone(),
            messages,
            temperature: self.provider.temperature,
            stream: true,
            tools: (!tools.is_empty()).then_some(tools),
            chat_template_kwargs: taotoken_glm_chat_template_kwargs(&self.provider),
        };
        let request = apply_provider_body_options(
            serde_json::to_value(request)?,
            &self.provider,
            ThinkingProtocol::OpenAiChat,
        )?;
        let url = format!(
            "{}/chat/completions",
            self.provider.base_url.trim_end_matches('/')
        );
        let headers = bearer_request_headers(&self.api_key, &[]);
        let mut debug =
            self.start_http_debug("POST", &url, "openai-chat", &headers, &request);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;
        let status = response.status();
        if let Some(debug) = debug.as_ref() {
            let _ = debug.write_response_headers(status.as_u16(), response.headers());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if let Some(debug) = debug.as_ref() {
                let _ = debug.finish_error(status.as_u16(), &body);
            }
            bail!(
                "{} ({status}): {body}",
                t("chat completions stream request failed", "聊天流式请求失败",)
            );
        }

        // 按字节缓冲再按行解码，避免多字节 UTF-8 被 chunk 切断后变成 U+FFFD
        let mut buffer = Utf8LineBuffer::default();
        let mut content = String::new();
        let mut content_emitted = 0usize;
        let mut reasoning = String::new();
        let mut reasoning_emitted = 0usize;
        let mut usage = None;
        let mut tool_calls = ToolCallAccumulator::default();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            for line in buffer.push(&chunk)? {
                if let Some(debug) = debug.as_mut() {
                    debug.append_stream_line(&line);
                }
                if let Some(done) = handle_sse_line(
                    &line,
                    &mut content,
                    &mut content_emitted,
                    &mut reasoning,
                    &mut reasoning_emitted,
                    &mut usage,
                    &mut tool_calls,
                    &mut on_event,
                )? {
                    if done {
                        let result = finalize_stream_result(
                            content,
                            reasoning,
                            usage,
                            tool_calls.finish(),
                        )?;
                        if let Some(debug) = debug.as_ref() {
                            let _ = debug.finish_ok(&result);
                        }
                        return Ok(result);
                    }
                }
            }
        }
        for line in buffer.finish()? {
            if let Some(debug) = debug.as_mut() {
                debug.append_stream_line(&line);
            }
            let _ = handle_sse_line(
                &line,
                &mut content,
                &mut content_emitted,
                &mut reasoning,
                &mut reasoning_emitted,
                &mut usage,
                &mut tool_calls,
                &mut on_event,
            )?;
        }
        let result = finalize_stream_result(content, reasoning, usage, tool_calls.finish())?;
        if let Some(debug) = debug.as_ref() {
            let _ = debug.finish_ok(&result);
        }
        Ok(result)
    }

    async fn chat_anthropic_stream<F>(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        on_event: &mut F,
    ) -> Result<ChatResult>
    where
        F: FnMut(ChatStreamEvent) -> Result<()>,
    {
        let request = AnthropicRequest {
            model: self.provider.default_model.clone(),
            system: lower_anthropic_system(&messages),
            messages: lower_anthropic_messages(messages),
            tools: (!tools.is_empty()).then(|| lower_anthropic_tools(tools)),
            stream: true,
            max_tokens: self.provider.anthropic_max_tokens,
            temperature: Some(self.provider.temperature),
        };
        let request = apply_provider_body_options(
            serde_json::to_value(request)?,
            &self.provider,
            ThinkingProtocol::Anthropic,
        )?;
        let url = format!("{}/messages", self.provider.base_url.trim_end_matches('/'));
        let headers = anthropic_request_headers(&self.api_key);
        let mut debug =
            self.start_http_debug("POST", &url, "anthropic", &headers, &request);
        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await?;
        let status = response.status();
        if let Some(debug) = debug.as_ref() {
            let _ = debug.write_response_headers(status.as_u16(), response.headers());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if let Some(debug) = debug.as_ref() {
                let _ = debug.finish_error(status.as_u16(), &body);
            }
            bail!(
                "{} ({status}): {body}",
                t(
                    "anthropic messages stream request failed",
                    "Anthropic Messages 流式请求失败"
                )
            );
        }

        let mut state = AnthropicStreamState::default();
        let mut buffer = SseDataBuffer::default();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            for data in buffer.push(&chunk)? {
                if let Some(debug) = debug.as_mut() {
                    // Anthropic 聚合后的 data 载荷，写成 SSE data 行便于回放
                    debug.append_stream_line(&format!("data: {data}"));
                    debug.append_stream_line("");
                }
                if handle_anthropic_sse_data(&data, &mut state, &mut *on_event)? {
                    let result = finalize_stream_result(
                        state.content,
                        state.reasoning,
                        state.usage,
                        state.tool_calls.finish(),
                    )?;
                    if let Some(debug) = debug.as_ref() {
                        let _ = debug.finish_ok(&result);
                    }
                    return Ok(result);
                }
            }
        }
        for data in buffer.finish()? {
            if let Some(debug) = debug.as_mut() {
                debug.append_stream_line(&format!("data: {data}"));
                debug.append_stream_line("");
            }
            let _ = handle_anthropic_sse_data(&data, &mut state, &mut *on_event)?;
        }
        let result = finalize_stream_result(
            state.content,
            state.reasoning,
            state.usage,
            state.tool_calls.finish(),
        )?;
        if let Some(debug) = debug.as_ref() {
            let _ = debug.finish_ok(&result);
        }
        Ok(result)
    }

    async fn chat_responses_stream<F>(
        &self,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        on_event: &mut F,
    ) -> Result<Option<ChatResult>>
    where
        F: FnMut(ChatStreamEvent) -> Result<()>,
    {
        let request = ResponsesRequest {
            model: self.provider.default_model.clone(),
            input: lower_responses_messages(messages),
            instructions: None,
            stream: true,
            tools: (!tools.is_empty()).then(|| lower_responses_tools(tools)),
            reasoning: Some(ResponsesReasoning {
                effort: Some("medium"),
                summary: Some("concise"),
            }),
            temperature: Some(self.provider.temperature),
        };
        let request = apply_provider_body_options(
            serde_json::to_value(request)?,
            &self.provider,
            ThinkingProtocol::OpenAiResponses,
        )?;
        let url = format!("{}/responses", self.provider.base_url.trim_end_matches('/'));
        let headers = bearer_request_headers(&self.api_key, &[]);
        let mut debug =
            self.start_http_debug("POST", &url, "openai-responses", &headers, &request);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;
        let status = response.status();
        if let Some(debug) = debug.as_ref() {
            let _ = debug.write_response_headers(status.as_u16(), response.headers());
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if responses_unsupported(status.as_u16(), &body) {
                return Ok(None);
            }
            if let Some(debug) = debug.as_ref() {
                let _ = debug.finish_error(status.as_u16(), &body);
            }
            bail!(
                "{} ({status}): {body}",
                t("responses stream request failed", "Responses 流式请求失败")
            );
        }

        let mut buffer = Utf8LineBuffer::default();
        let mut content = String::new();
        let mut content_emitted = 0usize;
        let mut reasoning = String::new();
        let mut reasoning_emitted = 0usize;
        let mut usage = None;
        let mut content_started = false;
        let mut tool_calls = ResponsesToolAccumulator::default();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            for line in buffer.push(&chunk)? {
                if let Some(debug) = debug.as_mut() {
                    debug.append_stream_line(&line);
                }
                if handle_responses_sse_line(
                    &line,
                    &mut content,
                    &mut content_emitted,
                    &mut reasoning,
                    &mut reasoning_emitted,
                    &mut usage,
                    &mut content_started,
                    &mut tool_calls,
                    &mut *on_event,
                )? {
                    let result =
                        finalize_stream_result(content, reasoning, usage, tool_calls.finish())?;
                    if let Some(debug) = debug.as_ref() {
                        let _ = debug.finish_ok(&result);
                    }
                    return Ok(Some(result));
                }
            }
        }
        for line in buffer.finish()? {
            if let Some(debug) = debug.as_mut() {
                debug.append_stream_line(&line);
            }
            let _ = handle_responses_sse_line(
                &line,
                &mut content,
                &mut content_emitted,
                &mut reasoning,
                &mut reasoning_emitted,
                &mut usage,
                &mut content_started,
                &mut tool_calls,
                &mut *on_event,
            )?;
        }
        let result = finalize_stream_result(content, reasoning, usage, tool_calls.finish())?;
        if let Some(debug) = debug.as_ref() {
            let _ = debug.finish_ok(&result);
        }
        Ok(Some(result))
    }

    fn uses_openai_responses(&self) -> bool {
        let model = self.provider.default_model.to_ascii_lowercase();
        model.starts_with("gpt-5")
            || model.starts_with("o1")
            || model.starts_with("o3")
            || model.starts_with("o4")
    }
}
