use crate::i18n::text as t;
use crate::render::stream::StreamRenderOptions;
use crate::render::style::TOOL_BULLET;

/// 归一化流式文本换行。
///
/// 参数:
/// - `text`: 原始文本
///
/// 返回:
/// - 使用 `\n` 的文本
pub(crate) fn normalize_stream_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// 判断工具调用自身是否已经有可见块展示。
///
/// 参数:
/// - `name`: 工具名称
///
/// 返回:
/// - 是否已经有命令块或 diff 块展示
pub(crate) fn tool_call_has_visible_block(name: &str) -> bool {
    matches!(name, "run_command" | "edit_file" | "write_file")
}

/// 生成工具状态事件文本。
///
/// 参数:
/// - `name`: 工具展示名称
/// - `status`: 状态文本
///
/// 返回:
/// - 工具状态事件文本
pub(crate) fn tool_event_text(name: &str, status: &str) -> String {
    format!("{TOOL_BULLET} {}: {name} {status}", t("tool", "工具"))
}

/// 生成等待动效详情行。
///
/// 参数:
/// - `options`: 流式渲染附加选项
///
/// 返回:
/// - 需要显示的详情行，没有可显示内容时返回空
pub(crate) fn wait_spinner_detail_line(options: &StreamRenderOptions) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(model) = options
        .wait_model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("{}: {model}", t("model", "模型")));
    }
    if let Some(level) = options
        .wait_thinking_level
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(format!("{}: {level}", t("thinking level", "思考等级")));
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}
