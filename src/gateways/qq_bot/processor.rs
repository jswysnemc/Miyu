use super::auth::QqBotAuthenticator;
use super::event::{QqBotInboundMediaKind, QqBotMessageEvent};
use super::inbound_media::{save_inbound_media, saved_image_to_data_url, SavedQqInboundMedia};
use super::prompt::channel_prompt;
use crate::agent::{Agent, AgentMode};
use crate::cli::build_tool_registry;
use crate::config::AppConfig;
use crate::gateways::channel_context::{save_latest_channel_context, ChannelContext};
use crate::gateways::channel_tools::{register_channel_message_tool, ActiveChannelTarget};
use crate::gateways::command_intercept::handle_gateway_command;
use crate::gateways::message::OutboundMessage;
use crate::gateways::qq_official::{QqOfficialClient, QqTargetKind};
use crate::llm::OpenAiCompatibleClient;
use crate::paths::MiyuPaths;
use crate::state::StateStore;
use anyhow::Result;
use tokio::sync::Mutex;

pub(crate) struct QqBotProcessorConfig {
    pub(crate) base_url: String,
    pub(crate) app_id: String,
    pub(crate) client_secret: String,
    pub(crate) verbose: bool,
}

pub(crate) struct QqBotProcessor {
    paths: MiyuPaths,
    base_url: String,
    verbose: bool,
    authenticator: Mutex<QqBotAuthenticator>,
    http_client: reqwest::Client,
    agent_lock: Mutex<()>,
}

impl QqBotProcessor {
    /// 创建 QQ 官方机器人消息处理器。
    ///
    /// 参数:
    /// - `paths`: Miyu 路径
    /// - `config`: QQ 官方机器人处理配置
    ///
    /// 返回:
    /// - QQ 官方机器人消息处理器
    pub(crate) fn new(paths: &MiyuPaths, config: QqBotProcessorConfig) -> Self {
        Self {
            paths: paths.clone(),
            base_url: config.base_url,
            verbose: config.verbose,
            authenticator: Mutex::new(QqBotAuthenticator::new(config.app_id, config.client_secret)),
            http_client: reqwest::Client::new(),
            agent_lock: Mutex::new(()),
        }
    }

    /// 输出 QQ 网关调试日志。
    ///
    /// 参数:
    /// - `message`: 日志内容
    ///
    /// 返回:
    /// - 无
    pub(crate) fn debug_log(&self, message: impl AsRef<str>) {
        if self.verbose {
            eprintln!("【QQ网关】【调试】{}", message.as_ref());
        }
    }

    /// 处理一条 QQ 官方机器人消息事件。
    ///
    /// 参数:
    /// - `event`: QQ 消息事件
    ///
    /// 返回:
    /// - 处理是否成功
    pub(crate) async fn handle_message_event(&self, event: QqBotMessageEvent) -> Result<()> {
        let _guard = self.agent_lock.lock().await;
        let context = ChannelContext::qq(
            event.target_kind,
            event.target_id.clone(),
            Some(event.msg_id.clone()),
        );
        save_latest_channel_context(&self.paths, &context)?;
        let authorization = self.authenticator.lock().await.authorization().await?;
        let client = QqOfficialClient::new(
            self.base_url.clone(),
            authorization,
            event.target_kind,
            event.target_id.clone(),
        );
        if let Some(reply) = handle_gateway_command(&self.paths, &event.prompt).await? {
            let message = OutboundMessage {
                text: Some(reply),
                media: Vec::new(),
            };
            client.send(&message, Some(&event.msg_id)).await?;
            return Ok(());
        }
        let (prompt, image_url) = self.prepare_agent_input(&event).await?;
        let reply = self
            .run_agent(client.clone(), &event, &context, prompt, image_url)
            .await?;
        if reply.trim().is_empty() {
            return Ok(());
        }
        let message = OutboundMessage {
            text: Some(reply),
            media: Vec::new(),
        };
        client.send(&message, Some(&event.msg_id)).await?;
        Ok(())
    }

    /// 准备 Agent 输入文本和首张图片。
    ///
    /// 参数:
    /// - `event`: QQ 消息事件
    ///
    /// 返回:
    /// - Agent 文本输入和可选图片 data URL
    async fn prepare_agent_input(
        &self,
        event: &QqBotMessageEvent,
    ) -> Result<(String, Option<String>)> {
        let mut prompt = event.prompt.clone();
        let mut image_url = None;
        for media in &event.media {
            match save_inbound_media(&self.paths, &self.http_client, media).await {
                Ok(saved) => {
                    append_saved_media_prompt(&mut prompt, &saved);
                    self.debug_log(format!(
                        "入站附件已保存 kind={} name={} mime={} path={}",
                        inbound_media_name(saved.kind),
                        saved.name,
                        saved.mime_type,
                        saved.path.display()
                    ));
                    if saved.kind == QqBotInboundMediaKind::Image && image_url.is_none() {
                        match saved_image_to_data_url(&saved) {
                            Ok(data_url) => image_url = Some(data_url),
                            Err(err) => {
                                prompt.push_str(&format!("\n\n图片视觉输入准备失败: {err}"));
                            }
                        }
                    }
                }
                Err(err) => {
                    self.debug_log(format!(
                        "入站附件保存失败 kind={} source={} error={err:#}",
                        inbound_media_name(media.kind),
                        media.source
                    ));
                    prompt.push_str(&format!(
                        "\n\n用户发送的{}保存失败: {err}\n来源: {}",
                        inbound_media_label(media.kind),
                        media.source
                    ));
                }
            }
        }
        Ok((prompt, image_url))
    }

    /// 运行 Miyu Agent 并返回回复文本。
    ///
    /// 参数:
    /// - `qq_client`: QQ 官方机器人客户端
    /// - `event`: QQ 消息事件
    /// - `prompt`: Agent 输入文本
    /// - `image_url`: 可选图片 data URL
    ///
    /// 返回:
    /// - Agent 回复文本
    async fn run_agent(
        &self,
        qq_client: QqOfficialClient,
        event: &QqBotMessageEvent,
        context: &ChannelContext,
        prompt: String,
        image_url: Option<String>,
    ) -> Result<String> {
        AppConfig::init_files(&self.paths)?;
        let config = AppConfig::load_or_default(&self.paths)?;
        let state = StateStore::new(&self.paths)?;
        state.init_files()?;
        let client = OpenAiCompatibleClient::from_config(&config, &self.paths)?;
        let mut registry = build_tool_registry(&config, &self.paths, AgentMode::Yolo)?;
        register_channel_message_tool(
            &mut registry,
            self.paths.clone(),
            config.clone(),
            ActiveChannelTarget::Qq {
                client: qq_client,
                msg_id: Some(event.msg_id.clone()),
            },
        );
        let progressive_loading_enabled = config.tools.progressive_loading_enabled;
        let prompt = format!("{}\n\n{}", context.inbound_marker(), prompt);
        let mut agent = Agent::new_with_extra_system_prompt(
            config,
            &self.paths,
            state.clone(),
            client,
            registry,
            AgentMode::Yolo,
            Some(channel_prompt()),
        )?;
        if progressive_loading_enabled {
            let mut loaded_tools = state.load_loaded_tools()?;
            loaded_tools.push("send_channel_message".to_string());
            agent.restore_loaded_tools(&loaded_tools);
        }
        let result = agent
            .chat_stream_with_image(&prompt, image_url, |_| Ok(()))
            .await?;
        if progressive_loading_enabled {
            state.save_loaded_tools(&agent.loaded_tools())?;
        }
        Ok(result.content)
    }
}

/// 将已保存媒体信息追加到 Agent 输入。
///
/// 参数:
/// - `prompt`: Agent 输入文本
/// - `saved`: 已保存媒体信息
///
/// 返回:
/// - 无
fn append_saved_media_prompt(prompt: &mut String, saved: &SavedQqInboundMedia) {
    prompt.push_str(&format!(
        "\n\n用户发送了{}: {}\n已保存到: {}\n媒体类型: {}",
        inbound_media_label(saved.kind),
        saved.name,
        saved.path.display(),
        saved.mime_type
    ));
}

/// 返回 QQ 目标类型名称。
///
/// 参数:
/// - `kind`: QQ 目标类型
///
/// 返回:
/// - 类型名称
pub(crate) fn target_kind_name(kind: QqTargetKind) -> &'static str {
    match kind {
        QqTargetKind::User => "user",
        QqTargetKind::Group => "group",
    }
}

/// 返回入站媒体中文名称。
///
/// 参数:
/// - `kind`: 入站媒体类型
///
/// 返回:
/// - 媒体类型名称
fn inbound_media_label(kind: QqBotInboundMediaKind) -> &'static str {
    match kind {
        QqBotInboundMediaKind::Image => "图片",
        QqBotInboundMediaKind::Voice => "语音",
        QqBotInboundMediaKind::Video => "视频",
        QqBotInboundMediaKind::File => "文件",
    }
}

/// 返回入站媒体日志名称。
///
/// 参数:
/// - `kind`: 入站媒体类型
///
/// 返回:
/// - 媒体类型文本
fn inbound_media_name(kind: QqBotInboundMediaKind) -> &'static str {
    match kind {
        QqBotInboundMediaKind::Image => "image",
        QqBotInboundMediaKind::Voice => "voice",
        QqBotInboundMediaKind::Video => "video",
        QqBotInboundMediaKind::File => "file",
    }
}
