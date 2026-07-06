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
        // 样式
        "styles/tokens.css" => css(include_str!("../../web/styles/tokens.css")),
        "styles/layout.css" => css(include_str!("../../web/styles/layout.css")),
        "styles/sidebar.css" => css(include_str!("../../web/styles/sidebar.css")),
        "styles/chat.css" => css(include_str!("../../web/styles/chat.css")),
        "styles/markdown.css" => css(include_str!("../../web/styles/markdown.css")),
        "styles/tools.css" => css(include_str!("../../web/styles/tools.css")),
        "styles/composer.css" => css(include_str!("../../web/styles/composer.css")),
        "styles/inspector.css" => css(include_str!("../../web/styles/inspector.css")),
        "styles/settings.css" => css(include_str!("../../web/styles/settings.css")),
        "styles/overlays.css" => css(include_str!("../../web/styles/overlays.css")),
        "styles/responsive.css" => css(include_str!("../../web/styles/responsive.css")),
        "styles/toast.css" => css(include_str!("../../web/styles/toast.css")),
        "styles/custom-select.css" => css(include_str!("../../web/styles/custom-select.css")),
        "styles/tag-editor.css" => css(include_str!("../../web/styles/tag-editor.css")),
        "styles/diff.css" => css(include_str!("../../web/styles/diff.css")),
        "styles/terminal.css" => css(include_str!("../../web/styles/terminal.css")),
        // 根模块
        "src/main.js" => js(include_str!("../../web/src/main.js")),
        "src/state.js" => js(include_str!("../../web/src/state.js")),
        "src/api.js" => js(include_str!("../../web/src/api.js")),
        "src/icons.js" => js(include_str!("../../web/src/icons.js")),
        "src/dom.js" => js(include_str!("../../web/src/dom.js")),
        "src/theme.js" => js(include_str!("../../web/src/theme.js")),
        "src/stream.js" => js(include_str!("../../web/src/stream.js")),
        // components 自研组件模块
        "src/components/toast.js" => js(include_str!("../../web/src/components/toast.js")),
        "src/components/custom-select.js" => {
            js(include_str!("../../web/src/components/custom-select.js"))
        }
        "src/components/tag-editor.js" => {
            js(include_str!("../../web/src/components/tag-editor.js"))
        }
        // markdown 模块
        "src/markdown/math.js" => js(include_str!("../../web/src/markdown/math.js")),
        "src/markdown/code.js" => js(include_str!("../../web/src/markdown/code.js")),
        "src/markdown/mermaid.js" => js(include_str!("../../web/src/markdown/mermaid.js")),
        "src/markdown/renderer.js" => js(include_str!("../../web/src/markdown/renderer.js")),
        "src/markdown/stream-renderer.js" => {
            js(include_str!("../../web/src/markdown/stream-renderer.js"))
        }
        // chat 模块
        "src/chat/view.js" => js(include_str!("../../web/src/chat/view.js")),
        "src/chat/message-node.js" => js(include_str!("../../web/src/chat/message-node.js")),
        "src/chat/actions.js" => js(include_str!("../../web/src/chat/actions.js")),
        "src/chat/tool-inline.js" => js(include_str!("../../web/src/chat/tool-inline.js")),
        // tools 渲染器模块
        "src/tools/common.js" => js(include_str!("../../web/src/tools/common.js")),
        "src/tools/registry.js" => js(include_str!("../../web/src/tools/registry.js")),
        "src/tools/default.js" => js(include_str!("../../web/src/tools/default.js")),
        "src/tools/file-tools.js" => js(include_str!("../../web/src/tools/file-tools.js")),
        "src/tools/command.js" => js(include_str!("../../web/src/tools/command.js")),
        "src/tools/web.js" => js(include_str!("../../web/src/tools/web.js")),
        "src/tools/image.js" => js(include_str!("../../web/src/tools/image.js")),
        "src/tools/memory.js" => js(include_str!("../../web/src/tools/memory.js")),
        "src/tools/kb.js" => js(include_str!("../../web/src/tools/kb.js")),
        "src/tools/meme.js" => js(include_str!("../../web/src/tools/meme.js")),
        "src/tools/divination.js" => js(include_str!("../../web/src/tools/divination.js")),
        "src/tools/system.js" => js(include_str!("../../web/src/tools/system.js")),
        "src/tools/heavy.js" => js(include_str!("../../web/src/tools/heavy.js")),
        "src/tools/load.js" => js(include_str!("../../web/src/tools/load.js")),
        "src/tools/diff-viewer.js" => js(include_str!("../../web/src/tools/diff-viewer.js")),
        "src/tools/terminal-viewer.js" => {
            js(include_str!("../../web/src/tools/terminal-viewer.js"))
        }
        // composer 模块
        "src/composer/input.js" => js(include_str!("../../web/src/composer/input.js")),
        "src/composer/attachments.js" => js(include_str!("../../web/src/composer/attachments.js")),
        "src/composer/controls.js" => js(include_str!("../../web/src/composer/controls.js")),
        // sessions 模块
        "src/sessions/list.js" => js(include_str!("../../web/src/sessions/list.js")),
        "src/sessions/search.js" => js(include_str!("../../web/src/sessions/search.js")),
        "src/sessions/manage.js" => js(include_str!("../../web/src/sessions/manage.js")),
        // inspector 模块
        "src/inspector/drawer.js" => js(include_str!("../../web/src/inspector/drawer.js")),
        "src/inspector/gateways.js" => js(include_str!("../../web/src/inspector/gateways.js")),
        "src/inspector/tasks.js" => js(include_str!("../../web/src/inspector/tasks.js")),
        // settings 模块
        "src/settings/modal.js" => js(include_str!("../../web/src/settings/modal.js")),
        "src/settings/providers.js" => js(include_str!("../../web/src/settings/providers.js")),
        "src/settings/tools.js" => js(include_str!("../../web/src/settings/tools.js")),
        "src/settings/gateways.js" => js(include_str!("../../web/src/settings/gateways.js")),
        "src/settings/form-utils.js" => js(include_str!("../../web/src/settings/form-utils.js")),
        "src/settings/provider-editor.js" => {
            js(include_str!("../../web/src/settings/provider-editor.js"))
        }
        // overlays 模块
        "src/overlays/lightbox.js" => js(include_str!("../../web/src/overlays/lightbox.js")),
        "src/overlays/mermaid-viewer.js" => {
            js(include_str!("../../web/src/overlays/mermaid-viewer.js"))
        }
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
