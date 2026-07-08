use crate::config::AppConfig;
use crate::paths::MiyuPaths;
use crate::state::StateStore;
use crate::tools::command::{
    list_background_tasks_for_user, start_gateway_background_task_for_user,
    stop_background_task_for_user,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;

const GATEWAY_QQ: &str = "qq";
const GATEWAY_WEIXIN: &str = "weixin";
const GATEWAY_LABEL_PREFIX: &str = "gateway:";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ManagedGateway {
    Qq,
    Weixin,
}

#[derive(Debug, Clone)]
pub(crate) struct GatewayRuntimeStatus {
    pub(crate) gateway: ManagedGateway,
    pub(crate) enabled: bool,
    pub(crate) task_id: Option<String>,
    pub(crate) status: String,
    pub(crate) pid: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct BackgroundTaskList {
    tasks: Vec<BackgroundTask>,
}

#[derive(Debug, Clone, Deserialize)]
struct BackgroundTask {
    id: String,
    label: String,
    runtime_owner_kind: Option<String>,
    runtime_owner_id: Option<String>,
    pid: u32,
    status: String,
}

impl ManagedGateway {
    /// 返回网关 ID。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 网关 ID
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Qq => GATEWAY_QQ,
            Self::Weixin => GATEWAY_WEIXIN,
        }
    }

    /// 返回网关显示名称。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 网关显示名称
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Qq => "QQ official bot",
            Self::Weixin => "Weixin iLink bot",
        }
    }

    /// 返回后台任务标签。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 后台任务标签
    fn label(self) -> String {
        format!("{GATEWAY_LABEL_PREFIX}{}", self.id())
    }

    /// 返回启动命令参数。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 命令参数列表
    fn command_args(self) -> &'static [&'static str] {
        match self {
            Self::Qq => &["gateway", "qq-bot"],
            Self::Weixin => &["gateway", "weixin-server"],
        }
    }
}

/// 返回所有受管理网关。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 网关列表
pub(crate) fn managed_gateways() -> &'static [ManagedGateway] {
    &[ManagedGateway::Qq, ManagedGateway::Weixin]
}

/// 查询所有网关运行状态。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
///
/// 返回:
/// - 网关运行状态列表
pub(crate) async fn gateway_runtime_statuses(
    paths: &MiyuPaths,
    config: &AppConfig,
) -> Result<Vec<GatewayRuntimeStatus>> {
    let tasks = gateway_background_tasks(paths, config).await?;
    Ok(managed_gateways()
        .iter()
        .map(|gateway| {
            let task = find_gateway_task(&tasks, *gateway);
            GatewayRuntimeStatus {
                gateway: *gateway,
                enabled: gateway_enabled(config, *gateway),
                task_id: task.map(|task| task.id.clone()),
                status: task
                    .map(|task| task.status.clone())
                    .unwrap_or_else(|| "stopped".to_string()),
                pid: task.map(|task| task.pid),
            }
        })
        .collect())
}

/// 启动指定网关后台任务。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
/// - `gateway`: 网关
///
/// 返回:
/// - JSON 格式启动结果
pub(crate) async fn start_gateway(
    paths: &MiyuPaths,
    config: &AppConfig,
    gateway: ManagedGateway,
) -> Result<String> {
    if find_gateway_task(&gateway_background_tasks(paths, config).await?, gateway)
        .map(|task| task.status == "running")
        .unwrap_or(false)
    {
        return Ok(serde_json::to_string_pretty(&json!({
            "ok": true,
            "already_running": true,
            "gateway": gateway.id(),
        }))?);
    }
    let command = gateway_command(gateway)?;
    start_gateway_background_task_for_user(
        paths,
        config,
        &command,
        Some(&gateway_workspace(paths)),
        Some(&gateway.label()),
        Some(0),
        gateway.id(),
    )
}

/// 停止指定网关后台任务。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
/// - `gateway`: 网关
///
/// 返回:
/// - 停止的任务数量
pub(crate) async fn stop_gateway(
    paths: &MiyuPaths,
    config: &AppConfig,
    gateway: ManagedGateway,
) -> Result<usize> {
    let tasks = gateway_background_tasks(paths, config).await?;
    let matching = tasks
        .iter()
        .filter(|task| is_gateway_task(task, gateway) && task.status == "running")
        .cloned()
        .collect::<Vec<_>>();
    if !matching.is_empty() {
        // 1. 先按 runtime owner 执行连接关闭策略，再让兼容 JSON 任务状态跟随刷新
        let state = StateStore::new(paths)?;
        state.apply_gateway_connection_close_policy(gateway.id())?;
    }
    for task in &matching {
        stop_background_task_for_user(paths, config, &task.id, false).await?;
    }
    Ok(matching.len())
}

/// 查询后台任务列表。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `config`: 应用配置
///
/// 返回:
/// - 后台任务列表
async fn gateway_background_tasks(
    paths: &MiyuPaths,
    config: &AppConfig,
) -> Result<Vec<BackgroundTask>> {
    let raw = list_background_tasks_for_user(paths, config).await?;
    let list = serde_json::from_str::<BackgroundTaskList>(&raw)?;
    Ok(list.tasks)
}

/// 查找网关对应后台任务。
///
/// 参数:
/// - `tasks`: 后台任务列表
/// - `gateway`: 网关
///
/// 返回:
/// - 可选后台任务
fn find_gateway_task(tasks: &[BackgroundTask], gateway: ManagedGateway) -> Option<&BackgroundTask> {
    tasks
        .iter()
        .filter(|task| is_gateway_task(task, gateway))
        .max_by_key(|task| &task.id)
}

/// 判断任务是否属于指定网关。
///
/// 参数:
/// - `task`: 后台任务
/// - `gateway`: 网关
///
/// 返回:
/// - 是否匹配
fn is_gateway_task(task: &BackgroundTask, gateway: ManagedGateway) -> bool {
    if task.runtime_owner_kind.is_some() || task.runtime_owner_id.is_some() {
        return task.runtime_owner_kind.as_deref() == Some("gateway")
            && task.runtime_owner_id.as_deref() == Some(gateway.id());
    }
    task.label == gateway.label()
}

/// 判断网关是否在配置中启用。
///
/// 参数:
/// - `config`: 应用配置
/// - `gateway`: 网关
///
/// 返回:
/// - 是否启用
fn gateway_enabled(config: &AppConfig, gateway: ManagedGateway) -> bool {
    match gateway {
        ManagedGateway::Qq => config.gateways.qq.enabled,
        ManagedGateway::Weixin => config.gateways.weixin.enabled,
    }
}

/// 组装网关启动命令。
///
/// 参数:
/// - `gateway`: 网关
///
/// 返回:
/// - shell 命令
fn gateway_command(gateway: ManagedGateway) -> Result<String> {
    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let mut parts = vec![shell_quote(&exe.display().to_string())];
    parts.extend(gateway.command_args().iter().map(|arg| shell_quote(arg)));
    Ok(parts.join(" "))
}

/// 返回网关工作目录。
///
/// 参数:
/// - `paths`: Miyu 路径
///
/// 返回:
/// - 工作目录字符串
fn gateway_workspace(paths: &MiyuPaths) -> String {
    paths.data_dir.join("workspace").display().to_string()
}

/// shell 单引号转义。
///
/// 参数:
/// - `value`: 原始文本
///
/// 返回:
/// - shell 安全文本
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证网关后台任务标签只匹配对应渠道。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn gateway_task_matching_uses_gateway_label() {
        let qq_task = task("1", "gateway:qq", "running", 100);
        let weixin_task = task("2", "gateway:weixin", "running", 101);

        assert!(is_gateway_task(&qq_task, ManagedGateway::Qq));
        assert!(!is_gateway_task(&weixin_task, ManagedGateway::Qq));
    }

    /// 验证网关后台任务优先使用 runtime owner 元数据匹配。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn gateway_task_matching_prefers_runtime_owner() {
        let mut task = task("1", "gateway:weixin", "running", 100);
        task.runtime_owner_kind = Some("gateway".to_string());
        task.runtime_owner_id = Some("qq".to_string());

        assert!(is_gateway_task(&task, ManagedGateway::Qq));
        assert!(!is_gateway_task(&task, ManagedGateway::Weixin));
    }

    /// 验证非网关 owner 不会因为标签被误判为网关任务。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn gateway_task_matching_rejects_labeled_non_gateway_owner() {
        let mut task = task("1", "gateway:qq", "running", 100);
        task.runtime_owner_kind = Some("session".to_string());
        task.runtime_owner_id = Some("default".to_string());

        assert!(!is_gateway_task(&task, ManagedGateway::Qq));
    }

    /// 验证查找网关任务时选择最新任务。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn find_gateway_task_selects_latest_matching_task() {
        let tasks = vec![
            task("100-1", "gateway:qq", "exited", 100),
            task("101-1", "gateway:qq", "running", 101),
            task("102-1", "gateway:weixin", "running", 102),
        ];

        let found = find_gateway_task(&tasks, ManagedGateway::Qq).unwrap();

        assert_eq!(found.id, "101-1");
        assert_eq!(found.status, "running");
    }

    /// 验证网关启动命令参数映射稳定。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn gateway_command_args_map_to_cli_subcommands() {
        assert_eq!(ManagedGateway::Qq.command_args(), &["gateway", "qq-bot"]);
        assert_eq!(
            ManagedGateway::Weixin.command_args(),
            &["gateway", "weixin-server"]
        );
    }

    /// 验证 shell 单引号转义。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("a'b"), "'a'\"'\"'b'");
    }

    /// 创建测试后台任务。
    ///
    /// 参数:
    /// - `id`: 任务 ID
    /// - `label`: 任务标签
    /// - `status`: 任务状态
    /// - `pid`: 进程 ID
    ///
    /// 返回:
    /// - 后台任务
    fn task(id: &str, label: &str, status: &str, pid: u32) -> BackgroundTask {
        BackgroundTask {
            id: id.to_string(),
            label: label.to_string(),
            runtime_owner_kind: None,
            runtime_owner_id: None,
            pid,
            status: status.to_string(),
        }
    }
}
