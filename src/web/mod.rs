mod api;
mod app_state;
mod assets;
mod auth;
mod error;
mod runs;
mod server;
mod services;
mod system_monitor;
mod terminal;
mod workspace;
mod workspaces;

use crate::cli::WebArgs;
use crate::paths::MiyuPaths;
use anyhow::Result;

/// 启动 Miyu Web 编程工作台。
///
/// 参数:
/// - `paths`: Miyu 路径集合
/// - `args`: Web 服务启动参数
///
/// 返回:
/// - 服务运行结果
pub(crate) async fn run(paths: &MiyuPaths, args: WebArgs) -> Result<()> {
    server::run(paths, args).await
}
