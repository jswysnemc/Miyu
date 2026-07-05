use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

/// 返回 Web 首页。
///
/// 参数:
/// - 无
///
/// 返回:
/// - HTML 响应
pub(crate) async fn index() -> Response {
    static_response(
        "text/html; charset=utf-8",
        include_str!("../../web/index.html"),
    )
}

/// 返回前端静态资源。
///
/// 参数:
/// - `path`: 资源路径
///
/// 返回:
/// - 静态资源响应
pub(crate) async fn asset(Path(path): Path<String>) -> Response {
    match path.as_str() {
        "styles/tokens.css" => css(include_str!("../../web/styles/tokens.css")),
        "styles/layout.css" => css(include_str!("../../web/styles/layout.css")),
        "styles/chat.css" => css(include_str!("../../web/styles/chat.css")),
        "styles/composer.css" => css(include_str!("../../web/styles/composer.css")),
        "styles/tools.css" => css(include_str!("../../web/styles/tools.css")),
        "styles/settings.css" => css(include_str!("../../web/styles/settings.css")),
        "styles/responsive.css" => css(include_str!("../../web/styles/responsive.css")),
        "src/api.js" => js(include_str!("../../web/src/api.js")),
        "src/state.js" => js(include_str!("../../web/src/state.js")),
        "src/icons.js" => js(include_str!("../../web/src/icons.js")),
        "src/chat-view.js" => js(include_str!("../../web/src/chat-view.js")),
        "src/tool-timeline.js" => js(include_str!("../../web/src/tool-timeline.js")),
        "src/sessions-view.js" => js(include_str!("../../web/src/sessions-view.js")),
        "src/settings-view.js" => js(include_str!("../../web/src/settings-view.js")),
        "src/gateways-view.js" => js(include_str!("../../web/src/gateways-view.js")),
        "src/composer.js" => js(include_str!("../../web/src/composer.js")),
        "src/stream.js" => js(include_str!("../../web/src/stream.js")),
        "src/main.js" => js(include_str!("../../web/src/main.js")),
        _ => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

/// 返回 CSS 响应。
///
/// 参数:
/// - `body`: CSS 内容
///
/// 返回:
/// - HTTP 响应
fn css(body: &'static str) -> Response {
    static_response("text/css; charset=utf-8", body)
}

/// 返回 JavaScript 响应。
///
/// 参数:
/// - `body`: JavaScript 内容
///
/// 返回:
/// - HTTP 响应
fn js(body: &'static str) -> Response {
    static_response("text/javascript; charset=utf-8", body)
}

/// 构造静态文本响应。
///
/// 参数:
/// - `content_type`: Content-Type
/// - `body`: 响应内容
///
/// 返回:
/// - HTTP 响应
fn static_response(content_type: &'static str, body: &'static str) -> Response {
    ([(header::CONTENT_TYPE, content_type)], body).into_response()
}
