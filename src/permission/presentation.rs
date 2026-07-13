use super::PermissionRequest;
use serde_json::Value;

const DETAIL_CHAR_LIMIT: usize = 1600;

/// 面向用户展示的权限请求字段。
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct PermissionDetail {
    pub(crate) label: String,
    pub(crate) value: String,
}

/// 去除协议 JSON 后的权限请求展示模型。
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct PermissionPresentation {
    pub(crate) action: String,
    pub(crate) summary: String,
    pub(crate) details: Vec<PermissionDetail>,
}

impl PermissionPresentation {
    /// 从工具权限请求生成紧凑、可读的展示数据。
    ///
    /// 参数:
    /// - `request`: 原始权限请求
    ///
    /// 返回:
    /// - 不包含裸露 JSON 的展示模型
    pub(crate) fn from_request(request: &PermissionRequest) -> Self {
        let arguments = serde_json::from_str::<Value>(&request.arguments).ok();
        let action = readable_action(&request.tool).to_string();
        let summary = arguments
            .as_ref()
            .and_then(|value| primary_summary(&request.tool, value))
            .unwrap_or_else(|| request.tool.replace('_', " "));
        let details = arguments
            .as_ref()
            .map(collect_details)
            .unwrap_or_else(|| fallback_details(&request.arguments));
        Self {
            action,
            summary,
            details,
        }
    }
}

/// 返回工具对应的用户可读动作。
///
/// 参数:
/// - `tool`: 工具协议名称
///
/// 返回:
/// - 中文动作标签
fn readable_action(tool: &str) -> &'static str {
    match tool {
        "run_command" | "background_command" => "执行命令",
        "edit_file" | "apply_patch" => "修改文件",
        "trash_path" => "移入回收站",
        "create_directory" => "创建目录",
        "write_file" => "写入文件",
        _ => "执行工具",
    }
}

/// 提取权限请求头部使用的主要对象。
///
/// 参数:
/// - `tool`: 工具协议名称
/// - `arguments`: 已解析工具参数
///
/// 返回:
/// - 命令、路径或任务摘要
fn primary_summary(tool: &str, arguments: &Value) -> Option<String> {
    let fields: &[&str] = match tool {
        "run_command" | "background_command" => &["command", "cmd"],
        "edit_file" | "apply_patch" | "trash_path" | "write_file" => {
            &["path", "file", "target", "destination"]
        }
        _ => &["path", "command", "query", "task", "description"],
    };
    fields
        .iter()
        .find_map(|field| arguments.get(*field).and_then(Value::as_str))
        .map(compact_summary)
}

/// 将参数对象转换为逐项展示字段。
///
/// 参数:
/// - `arguments`: 已解析工具参数
///
/// 返回:
/// - 按常用字段顺序排列的展示项
fn collect_details(arguments: &Value) -> Vec<PermissionDetail> {
    let Some(object) = arguments.as_object() else {
        return vec![PermissionDetail {
            label: "参数".to_string(),
            value: display_value(arguments),
        }];
    };
    let preferred = [
        "command",
        "cmd",
        "cwd",
        "path",
        "file",
        "target",
        "destination",
        "patch",
        "content",
        "query",
        "task",
        "description",
    ];
    let mut details = Vec::new();
    for key in preferred {
        if let Some(value) = object.get(key) {
            push_detail(&mut details, key, value);
        }
    }
    for (key, value) in object {
        if preferred.contains(&key.as_str()) || key.starts_with("_miyu_") {
            continue;
        }
        push_detail(&mut details, key, value);
    }
    details
}

/// 追加一项非空参数详情。
///
/// 参数:
/// - `details`: 待写入的展示项集合
/// - `key`: 参数字段名
/// - `value`: 参数字段值
///
/// 返回:
/// - 无
fn push_detail(details: &mut Vec<PermissionDetail>, key: &str, value: &Value) {
    let value = display_value(value);
    if value.trim().is_empty() {
        return;
    }
    details.push(PermissionDetail {
        label: readable_field_label(key).to_string(),
        value,
    });
}

/// 将参数字段名转换为用户可读标签。
///
/// 参数:
/// - `key`: 参数字段名
///
/// 返回:
/// - 中文标签或原字段名
fn readable_field_label(key: &str) -> &str {
    match key {
        "command" | "cmd" => "命令",
        "cwd" => "工作目录",
        "path" | "file" => "文件",
        "target" => "目标",
        "destination" => "目标位置",
        "patch" => "变更",
        "content" => "内容",
        "query" => "查询",
        "task" => "任务",
        "description" => "说明",
        _ => key,
    }
}

/// 将 JSON 值转换为无外层 JSON 语法的可读文本。
///
/// 参数:
/// - `value`: 已解析参数值
///
/// 返回:
/// - 不含对象外框的文本
fn display_value(value: &Value) -> String {
    let text = match value {
        Value::Null => "空".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(display_value)
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| format!("{}: {}", readable_field_label(key), display_value(value)))
            .collect::<Vec<_>>()
            .join("\n"),
    };
    truncate_chars(&text, DETAIL_CHAR_LIMIT)
}

/// 为无法解析的旧请求保留纯文本参数，不输出 JSON 外框。
///
/// 参数:
/// - `arguments`: 无法解析的原始参数文本
///
/// 返回:
/// - 可选的纯文本参数展示项
fn fallback_details(arguments: &str) -> Vec<PermissionDetail> {
    let value = arguments
        .trim()
        .trim_matches(['{', '}'])
        .replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\"", "\"")
        .replace('"', "")
        .trim()
        .to_string();
    if value.is_empty() {
        Vec::new()
    } else {
        vec![PermissionDetail {
            label: "参数".to_string(),
            value: truncate_chars(&value, DETAIL_CHAR_LIMIT),
        }]
    }
}

/// 压缩权限请求头部摘要。
///
/// 参数:
/// - `value`: 原始摘要文本
///
/// 返回:
/// - 单行且长度受限的摘要
fn compact_summary(value: &str) -> String {
    truncate_chars(&value.split_whitespace().collect::<Vec<_>>().join(" "), 120)
}

/// 按字符数量截断文本。
///
/// 参数:
/// - `value`: 原始文本
/// - `limit`: 最大字符数量
///
/// 返回:
/// - 必要时带省略号的文本
fn truncate_chars(value: &str, limit: usize) -> String {
    let mut chars = value.chars();
    let mut output = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        output.push_str("…");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证命令权限请求不暴露 JSON 外框。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn command_request_exposes_command_without_json_wrapper() {
        let request = PermissionRequest {
            id: "id".to_string(),
            session_id: "session".to_string(),
            tool: "run_command".to_string(),
            arguments: r#"{"command":"cargo test","cwd":"/workspace"}"#.to_string(),
        };

        let presentation = PermissionPresentation::from_request(&request);

        assert_eq!(presentation.action, "执行命令");
        assert_eq!(presentation.summary, "cargo test");
        assert!(presentation
            .details
            .iter()
            .any(|detail| detail.label == "命令" && detail.value == "cargo test"));
        assert!(!presentation
            .details
            .iter()
            .any(|detail| detail.value.contains('{')));
    }
}
