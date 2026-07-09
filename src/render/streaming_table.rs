use crate::render::markdown_inline::render_table_cell_content;
use crate::render::streaming_replace::{clear_rendered_rows, raw_visual_rows};
use crate::render::table;

/// Markdown 表格流式替换状态。
pub(crate) struct StreamingTable {
    lines: Vec<String>,
    raw_visual_rows: usize,
    replace_streamed_rows: bool,
}

impl StreamingTable {
    /// 创建表格流式重绘状态。
    ///
    /// 返回:
    /// - 新的表格流式重绘状态
    pub(crate) fn new() -> Self {
        Self {
            lines: Vec::new(),
            raw_visual_rows: 0,
            replace_streamed_rows: true,
        }
    }

    /// 创建用于 source 重放的稳定表格渲染状态。
    ///
    /// 稳定模式在表格确认前不输出原始行，避免把光标上移替换序列写入可重放 transcript。
    ///
    /// 返回:
    /// - 不产生终端回退控制序列的表格状态
    pub(crate) fn new_stable() -> Self {
        Self {
            lines: Vec::new(),
            raw_visual_rows: 0,
            replace_streamed_rows: false,
        }
    }

    /// 判断当前是否缓存了表格行。
    ///
    /// 返回:
    /// - 是否存在缓存行
    pub(crate) fn is_active(&self) -> bool {
        !self.lines.is_empty()
    }

    /// 判断当前缓存是否已经确认是 Markdown 表格。
    ///
    /// 返回:
    /// - 第二行是分隔行时返回 true
    pub(crate) fn is_confirmed(&self) -> bool {
        self.lines
            .get(1)
            .map(String::as_str)
            .is_some_and(table::is_table_separator)
    }

    /// 推入一行表格候选文本并返回原始行。
    ///
    /// 参数:
    /// - `line`: 当前收到的 Markdown 行
    ///
    /// 返回:
    /// - 需要立即写入终端的原始 Markdown 行
    pub(crate) fn push_line(&mut self, line: &str) -> String {
        self.lines.push(line.to_string());
        self.raw_visual_rows += raw_visual_rows(line);
        if self.replace_streamed_rows {
            format!("{line}\n")
        } else {
            String::new()
        }
    }

    /// 结束当前表格并返回最终替换渲染。
    ///
    /// 返回:
    /// - 已确认表格的替换渲染文本，未确认表格返回空
    pub(crate) fn finish(&mut self) -> String {
        let output = if self.is_confirmed() {
            self.render_current()
        } else {
            String::new()
        };
        self.lines.clear();
        self.raw_visual_rows = 0;
        output
    }

    /// 按当前全部行计算列宽并替换原始行输出。
    ///
    /// 返回:
    /// - 包含清除原始行控制序列和最终表格文本的输出
    fn render_current(&mut self) -> String {
        let mut output = if self.replace_streamed_rows {
            // 1. 流式模式先清除已写入的原始 Markdown 行
            clear_rendered_rows(self.raw_visual_rows)
        } else {
            String::new()
        };
        // 2. source 重放与流式模式都按完整表格一次性计算列宽
        output.push_str(&table::render_table(&self.lines, render_table_cell_content));
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_raw_rows_before_final_render() {
        let mut table = StreamingTable::new();

        assert_eq!(table.push_line("| a | b |"), "| a | b |\n");
        assert_eq!(table.push_line("| - | - |"), "| - | - |\n");
    }

    #[test]
    fn replaces_raw_rows_with_rendered_table_on_finish() {
        let mut table = StreamingTable::new();

        assert_eq!(table.push_line("| 软件 | 命令 |"), "| 软件 | 命令 |\n");
        assert_eq!(table.push_line("|---|---|"), "|---|---|\n");
        assert_eq!(
            table.push_line("| Neovim | `sudo pacman -S neovim` |"),
            "| Neovim | `sudo pacman -S neovim` |\n"
        );
        let output = table.finish();

        assert!(output.starts_with("\x1b[1A\r\x1b[2K"));
        assert!(output.contains("sudo pacman -S neovim"));
    }

    #[test]
    fn stable_table_waits_for_confirmation_without_cursor_replacement() {
        let mut table = StreamingTable::new_stable();

        assert!(table.push_line("| a | b |").is_empty());
        assert!(table.push_line("| - | - |").is_empty());
        assert!(table.push_line("| 1 | 2 |").is_empty());
        let output = table.finish();

        assert!(output.contains('┌'));
        assert!(!output.contains("\x1b[1A"));
    }
}
