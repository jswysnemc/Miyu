use super::ToolCallStreamProgress;

const PROGRESS_BYTE_STEP: usize = 8 * 1024;
const ARGUMENTS_PREVIEW_CHARS: usize = 4096;
const TARGET_KEYS: &[&str] = &[
    "path",
    "name",
    "group_name",
    "tool_name",
    "include",
    "pattern",
];

#[derive(Debug, Default)]
pub(crate) struct ToolCallProgressTracker {
    entries: Vec<ToolCallProgressEntry>,
}

#[derive(Debug, Default)]
struct ToolCallProgressEntry {
    emitted: bool,
    last_name: String,
    last_arguments_bytes: usize,
    target_seen: bool,
}

impl ToolCallProgressTracker {
    /// 更新工具调用参数接收进度。
    ///
    /// 参数:
    /// - `index`: 工具调用索引
    /// - `name`: 当前已接收到的工具名称
    /// - `arguments`: 当前已接收到的完整参数片段
    ///
    /// 返回:
    /// - 需要向外发送的进度事件，没有新进度时返回空
    pub(crate) fn update(
        &mut self,
        index: usize,
        name: &str,
        arguments: &str,
    ) -> Option<ToolCallStreamProgress> {
        while self.entries.len() <= index {
            self.entries.push(ToolCallProgressEntry::default());
        }
        let entry = &mut self.entries[index];
        let arguments_bytes = arguments.len();
        let name_changed = !name.trim().is_empty() && entry.last_name != name;
        let size_changed =
            arguments_bytes.saturating_sub(entry.last_arguments_bytes) >= PROGRESS_BYTE_STEP;
        let target_seen = entry.target_seen || has_complete_target_field(arguments);
        let target_changed = target_seen && !entry.target_seen;
        let first_visible = !entry.emitted && (!name.trim().is_empty() || arguments_bytes > 0);
        if !(first_visible || name_changed || size_changed || target_changed) {
            return None;
        }
        entry.emitted = true;
        entry.last_name = name.to_string();
        entry.last_arguments_bytes = arguments_bytes;
        entry.target_seen = target_seen;
        Some(ToolCallStreamProgress {
            index,
            name: (!name.trim().is_empty()).then(|| name.to_string()),
            arguments_chars: arguments.chars().count(),
            arguments_bytes,
            arguments_preview: arguments.chars().take(ARGUMENTS_PREVIEW_CHARS).collect(),
        })
    }
}

/// 判断参数片段是否已经包含完整 target 字段。
///
/// 参数:
/// - `arguments`: 当前累计参数文本
///
/// 返回:
/// - 是否存在完整 target 字段
fn has_complete_target_field(arguments: &str) -> bool {
    TARGET_KEYS
        .iter()
        .any(|key| complete_json_string_field(arguments, key))
}

/// 判断 JSON 片段中指定字符串字段是否已经闭合。
///
/// 参数:
/// - `raw`: JSON 参数片段
/// - `key`: 字段名
///
/// 返回:
/// - 字符串字段是否完整
fn complete_json_string_field(raw: &str, key: &str) -> bool {
    let pattern = format!("\"{}\"", key);
    let Some(key_index) = raw.find(&pattern) else {
        return false;
    };
    let after_key = &raw[key_index + pattern.len()..];
    let Some(colon_index) = after_key.find(':') else {
        return false;
    };
    let after_colon = after_key[colon_index + 1..].trim_start();
    let Some(quote_index) = after_colon.find('"') else {
        return false;
    };
    json_string_is_closed(&after_colon[quote_index..])
}

/// 判断 JSON 字符串片段是否已经闭合。
///
/// 参数:
/// - `value`: 以双引号开头的 JSON 字符串片段
///
/// 返回:
/// - 是否找到未转义结束双引号
fn json_string_is_closed(value: &str) -> bool {
    if !value.starts_with('"') {
        return false;
    }
    let mut escaped = false;
    for ch in value.chars().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_emits_initial_name_and_large_argument_steps() {
        let mut tracker = ToolCallProgressTracker::default();

        let initial = tracker.update(0, "write_file", "").unwrap();
        assert_eq!(initial.name.as_deref(), Some("write_file"));
        assert_eq!(initial.arguments_bytes, 0);
        assert_eq!(initial.arguments_preview, "");

        assert!(tracker.update(0, "write_file", "abc").is_none());

        let large = "x".repeat(PROGRESS_BYTE_STEP);
        let next = tracker.update(0, "write_file", &large).unwrap();
        assert_eq!(next.arguments_bytes, PROGRESS_BYTE_STEP);
        assert_eq!(next.arguments_preview.len(), ARGUMENTS_PREVIEW_CHARS);
    }

    #[test]
    fn tracker_emits_when_target_field_is_complete() {
        let mut tracker = ToolCallProgressTracker::default();

        let initial = tracker.update(0, "write_file", "").unwrap();
        assert_eq!(initial.name.as_deref(), Some("write_file"));

        assert!(tracker
            .update(0, "write_file", r#"{"path":"src/mai"#)
            .is_none());

        let target = tracker
            .update(0, "write_file", r#"{"path":"src/main.rs","content":""#)
            .unwrap();
        assert_eq!(
            target.arguments_preview,
            r#"{"path":"src/main.rs","content":""#
        );
    }
}
