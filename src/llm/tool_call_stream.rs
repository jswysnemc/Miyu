use super::ToolCallStreamProgress;

const PROGRESS_BYTE_STEP: usize = 8 * 1024;

#[derive(Debug, Default)]
pub(crate) struct ToolCallProgressTracker {
    entries: Vec<ToolCallProgressEntry>,
}

#[derive(Debug, Default)]
struct ToolCallProgressEntry {
    emitted: bool,
    last_name: String,
    last_arguments_bytes: usize,
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
        let first_visible = !entry.emitted && (!name.trim().is_empty() || arguments_bytes > 0);
        if !(first_visible || name_changed || size_changed) {
            return None;
        }
        entry.emitted = true;
        entry.last_name = name.to_string();
        entry.last_arguments_bytes = arguments_bytes;
        Some(ToolCallStreamProgress {
            index,
            name: (!name.trim().is_empty()).then(|| name.to_string()),
            arguments_chars: arguments.chars().count(),
            arguments_bytes,
        })
    }
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

        assert!(tracker.update(0, "write_file", "abc").is_none());

        let large = "x".repeat(PROGRESS_BYTE_STEP);
        let next = tracker.update(0, "write_file", &large).unwrap();
        assert_eq!(next.arguments_bytes, PROGRESS_BYTE_STEP);
    }
}
