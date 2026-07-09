use crate::render::markdown_inline::render_table_cell_content;
use crate::render::streaming_replace::{clear_rendered_rows, raw_visual_rows};
use crate::render::table;

/// Markdown 表格流式替换状态。
pub(crate) struct StreamingTable {
    lines: Vec<String>,
    raw_visual_rows: usize,
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
        format!("{line}\n")
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
        // 1. 先清除流式阶段已经输出的原始 Markdown 行
        let mut output = clear_rendered_rows(self.raw_visual_rows);
        // 2. 再按完整表格一次性计算列宽并输出最终渲染
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
}
