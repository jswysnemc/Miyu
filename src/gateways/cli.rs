use super::message::{MediaKind, OutboundMedia, OutboundMessage};
use super::onebot::{OneBotClient, OneBotTargetKind};
use super::onebot_server::{run_onebot_server, OneBotServerConfig};
use super::qq_bot::webhook_server::{run_qq_bot_webhook_server, QqBotWebhookServerConfig};
use super::qq_official::{QqOfficialClient, QqTargetKind};
use super::wecom_webhook::WecomWebhookClient;
use super::weixin_bot::client::default_cdn_base_url as default_weixin_cdn_base_url;
use super::weixin_bot::login::{
    default_base_url as default_weixin_base_url, default_bot_type as default_weixin_bot_type,
    load_weixin_account, run_weixin_login, WeixinLoginConfig,
};
use super::weixin_bot::server::{run_weixin_bot_server, WeixinBotServerConfig};
use crate::paths::MiyuPaths;
use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use serde_json::Value;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub(crate) struct GatewayArgs {
    #[arg(long, short = 'v', global = true)]
    pub(crate) verbose: bool,
    #[command(subcommand)]
    pub(crate) command: GatewayCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum GatewayCommand {
    WecomWebhook(WecomWebhookArgs),
    QqOfficial(QqOfficialArgs),
    QqBotWebhook(QqBotWebhookArgs),
    Onebot(OneBotArgs),
    OnebotServer(OneBotServerArgs),
    WeixinLogin(WeixinLoginArgs),
    WeixinServer(WeixinServerArgs),
}

#[derive(Debug, Args)]
pub(crate) struct WecomWebhookArgs {
    #[arg(long)]
    pub(crate) webhook_url: String,
    #[arg(long)]
    pub(crate) text: Option<String>,
    #[arg(long)]
    pub(crate) image: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) file: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct QqOfficialArgs {
    #[arg(long, default_value = "https://api.sgroup.qq.com")]
    pub(crate) base_url: String,
    #[arg(long)]
    pub(crate) authorization: String,
    #[arg(long)]
    pub(crate) target_kind: String,
    #[arg(long)]
    pub(crate) target_id: String,
    #[arg(long)]
    pub(crate) msg_id: Option<String>,
    #[arg(long)]
    pub(crate) text: Option<String>,
    #[arg(long)]
    pub(crate) image: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) file: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct QqBotWebhookArgs {
    #[arg(long, default_value = "127.0.0.1:8766")]
    pub(crate) listen: SocketAddr,
    #[arg(long, default_value = "https://api.sgroup.qq.com")]
    pub(crate) base_url: String,
    #[arg(long)]
    pub(crate) app_id: String,
    #[arg(long)]
    pub(crate) client_secret: String,
}

#[derive(Debug, Args)]
pub(crate) struct OneBotArgs {
    #[arg(long, default_value = "http://127.0.0.1:3000")]
    pub(crate) base_url: String,
    #[arg(long)]
    pub(crate) access_token: Option<String>,
    #[arg(long)]
    pub(crate) target_kind: String,
    #[arg(long)]
    pub(crate) target_id: i64,
    #[arg(long)]
    pub(crate) text: Option<String>,
    #[arg(long)]
    pub(crate) image: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) file: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct OneBotServerArgs {
    #[arg(long, default_value = "127.0.0.1:8765")]
    pub(crate) listen: SocketAddr,
    #[arg(long, default_value = "http://127.0.0.1:3000")]
    pub(crate) onebot_base_url: String,
    #[arg(long)]
    pub(crate) access_token: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct WeixinServerArgs {
    #[arg(long, default_value_t = default_weixin_base_url().to_string())]
    pub(crate) base_url: String,
    #[arg(long, default_value_t = default_weixin_cdn_base_url().to_string())]
    pub(crate) cdn_base_url: String,
    #[arg(long)]
    pub(crate) token: Option<String>,
    #[arg(long)]
    pub(crate) account: Option<String>,
    #[arg(long)]
    pub(crate) bot_agent: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct WeixinLoginArgs {
    #[arg(long, default_value_t = default_weixin_base_url().to_string())]
    pub(crate) base_url: String,
    #[arg(long, default_value_t = default_weixin_bot_type().to_string())]
    pub(crate) bot_type: String,
    #[arg(long, default_value_t = 480)]
    pub(crate) timeout_secs: u64,
}

/// 运行消息网关命令。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `args`: 网关命令参数
///
/// 返回:
/// - 命令是否成功
pub(crate) async fn run_gateway(paths: &MiyuPaths, args: GatewayArgs) -> Result<()> {
    let verbose = args.verbose;
    enter_gateway_workspace(paths, verbose)?;
    let responses = match args.command {
        GatewayCommand::WecomWebhook(args) => {
            let message = outbound_message(args.text, args.image, args.file)?;
            let client = WecomWebhookClient::new(args.webhook_url);
            client.send(&message).await?
        }
        GatewayCommand::QqOfficial(args) => {
            let message = outbound_message(args.text, args.image, args.file)?;
            let target_kind = QqTargetKind::parse(&args.target_kind)?;
            let client = QqOfficialClient::new(
                args.base_url,
                args.authorization,
                target_kind,
                args.target_id,
            );
            client.send(&message, args.msg_id.as_deref()).await?
        }
        GatewayCommand::QqBotWebhook(args) => {
            run_qq_bot_webhook_server(
                paths,
                QqBotWebhookServerConfig {
                    listen: args.listen,
                    base_url: args.base_url,
                    app_id: args.app_id,
                    client_secret: args.client_secret,
                    verbose,
                },
            )
            .await?;
            return Ok(());
        }
        GatewayCommand::Onebot(args) => {
            let message = outbound_message(args.text, args.image, args.file)?;
            let target_kind = OneBotTargetKind::parse(&args.target_kind)?;
            let client = OneBotClient::new(
                args.base_url,
                args.access_token,
                target_kind,
                args.target_id,
            );
            client.send(&message).await?
        }
        GatewayCommand::OnebotServer(args) => {
            run_onebot_server(
                paths,
                OneBotServerConfig {
                    listen: args.listen,
                    onebot_base_url: args.onebot_base_url,
                    access_token: args.access_token,
                },
            )
            .await?;
            return Ok(());
        }
        GatewayCommand::WeixinLogin(args) => {
            run_weixin_login(
                paths,
                WeixinLoginConfig {
                    base_url: args.base_url,
                    bot_type: args.bot_type,
                    timeout_secs: args.timeout_secs,
                },
            )
            .await?;
            return Ok(());
        }
        GatewayCommand::WeixinServer(args) => {
            let (base_url, cdn_base_url, token) = match args.token {
                Some(token) if !token.trim().is_empty() => {
                    (args.base_url, args.cdn_base_url, token)
                }
                _ => {
                    let account = load_weixin_account(paths, args.account.as_deref())?;
                    (account.base_url, account.cdn_base_url, account.token)
                }
            };
            run_weixin_bot_server(
                paths,
                WeixinBotServerConfig {
                    base_url,
                    cdn_base_url,
                    token,
                    bot_agent: args.bot_agent,
                    verbose,
                },
            )
            .await?;
            return Ok(());
        }
    };
    println!("{}", serde_json::to_string_pretty(&responses)?);
    Ok(())
}

/// 切换到网关默认工作目录。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `verbose`: 是否输出详细日志
///
/// 返回:
/// - 工作目录是否切换成功
fn enter_gateway_workspace(paths: &MiyuPaths, verbose: bool) -> Result<()> {
    let workspace = paths.data_dir.join("workspace");
    std::fs::create_dir_all(&workspace)?;
    std::env::set_current_dir(&workspace)?;
    if verbose {
        eprintln!("【网关】【工作目录】{}", workspace.display());
    }
    Ok(())
}

/// 从命令行参数组装统一出站消息。
///
/// 参数:
/// - `text`: 文本消息
/// - `images`: 图片路径列表
/// - `files`: 文件路径列表
///
/// 返回:
/// - 统一出站消息
fn outbound_message(
    text: Option<String>,
    images: Vec<PathBuf>,
    files: Vec<PathBuf>,
) -> Result<OutboundMessage> {
    let mut message = OutboundMessage {
        text: text.filter(|value| !value.trim().is_empty()),
        media: Vec::new(),
    };
    for path in images {
        validate_file(&path)?;
        message.media.push(OutboundMedia {
            kind: MediaKind::Image,
            path,
        });
    }
    for path in files {
        validate_file(&path)?;
        message.media.push(OutboundMedia {
            kind: MediaKind::File,
            path,
        });
    }
    if message.is_empty() {
        bail!("provide --text, --image, or --file");
    }
    Ok(message)
}

/// 校验待发送文件存在且不是目录。
///
/// 参数:
/// - `path`: 文件路径
///
/// 返回:
/// - 文件是否有效
fn validate_file(path: &PathBuf) -> Result<()> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| anyhow::anyhow!("invalid media path {}: {err}", path.display()))?;
    if !metadata.is_file() {
        bail!("media path is not a file: {}", path.display());
    }
    Ok(())
}

#[allow(dead_code)]
fn _responses_are_json(_responses: &[Value]) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_outbound_message() {
        let err = outbound_message(None, Vec::new(), Vec::new()).unwrap_err();

        assert!(err.to_string().contains("provide --text"));
    }
}
