mod agent_options;
mod background_tasks;
mod config;
mod gateways;
mod health;
mod prompts;
mod providers;
mod runs;
mod sessions;
mod system;
mod terminal;
mod workspace;
mod workspaces;

use super::app_state::WebAppState;
use super::auth;
use axum::middleware;
use axum::routing::{get, post};
use axum::Router;

/// 组装公开与受保护 API 路由。
///
/// 参数:
/// - `state`: Web 应用状态
///
/// 返回:
/// - API 路由
pub(super) fn router(state: WebAppState) -> Router<WebAppState> {
    let protected = Router::new()
        .merge(workspaces::routes())
        .merge(config::routes())
        .merge(agent_options::routes())
        .merge(background_tasks::routes())
        .merge(providers::routes())
        .merge(prompts::routes())
        .merge(gateways::routes())
        .merge(sessions::routes())
        .merge(runs::routes())
        .merge(workspace::routes())
        .merge(system::routes())
        .merge(terminal::routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));
    Router::new()
        .route("/api/health", get(health::health))
        .route("/api/auth/session", post(auth::create_session))
        .merge(protected)
}
