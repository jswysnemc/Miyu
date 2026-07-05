use super::app_state::WebAppState;
use super::args::WebArgs;
use super::{chat_api, config_api, gateway_api, session_api, static_assets, tool_api};
use crate::config::AppConfig;
use crate::paths::MiyuPaths;
use anyhow::{Context, Result};
use axum::routing::{get, post, put};
use axum::Router;
use tokio::net::TcpListener;

/// 启动 Miyu Web 服务。
///
/// 参数:
/// - `paths`: Miyu 路径
/// - `args`: Web 命令参数
///
/// 返回:
/// - 服务运行结果
pub(crate) async fn run_web_server(paths: &MiyuPaths, args: WebArgs) -> Result<()> {
    AppConfig::init_files(paths)?;
    paths.create_dirs()?;
    let state = WebAppState::new(paths);
    let app = router(state);
    let listener = TcpListener::bind(args.listen)
        .await
        .with_context(|| format!("failed to bind Miyu Web server: {}", args.listen))?;
    println!("Miyu Web listening on http://{}", args.listen);
    axum::serve(listener, app).await?;
    Ok(())
}

/// 构造 Web 路由。
///
/// 参数:
/// - `state`: Web 应用共享状态
///
/// 返回:
/// - Axum 路由
fn router(state: WebAppState) -> Router {
    Router::new()
        .route("/", get(static_assets::index))
        .route("/assets/*path", get(static_assets::asset))
        .route("/api/health", get(config_api::health))
        .route("/api/config", get(config_api::get_config))
        .route("/api/config", put(config_api::save_config))
        .route("/api/sessions", get(session_api::list_sessions))
        .route("/api/sessions", post(session_api::create_session))
        .route(
            "/api/sessions/:session_id",
            get(session_api::get_session_history).delete(session_api::delete_session),
        )
        .route("/api/chat", post(chat_api::chat))
        .route("/api/gateways", get(gateway_api::list_gateways))
        .route(
            "/api/gateways/:gateway/start",
            post(gateway_api::start_gateway),
        )
        .route(
            "/api/gateways/:gateway/stop",
            post(gateway_api::stop_gateway),
        )
        .route(
            "/api/tools/background",
            get(tool_api::list_background_tasks),
        )
        .route(
            "/api/tools/background/:task_id/output",
            get(tool_api::read_background_output),
        )
        .with_state(state)
}
