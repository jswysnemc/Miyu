use super::viewport::InlineViewport;
use crate::cli::repl_chrome::{chrome_fixed_rows, chrome_rule, ReplChrome};
use crate::cli::repl_commands::{repl_command_suggestions, ReplCommandSuggestion};
use crate::cli::repl_input_render::{
    repl_cursor_position_for_cols, repl_line_rows_for_cols, repl_prompt_rows_for_cols,
    repl_visible_input_lines,
};
use crate::cli::repl_text::{repl_input_lines, take_chars, visible_width};
use crate::cli::REPL_MAX_VISIBLE_INPUT_ROWS;
use anyhow::Result;
use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use std::io::Write;

/// 可从输入 source 按当前终端宽度重绘的 REPL composer。
#[derive(Clone)]
pub(super) struct ComposerFrame {
    chrome: ReplChrome,
    input: String,
    cursor: usize,
    is_pasted: bool,
    slash_selection: usize,
}

const MAX_SLASH_PANEL_ITEMS: usize = 8;

impl ComposerFrame {
    /// 创建当前输入状态的 composer source。
    ///
    /// 参数:
    /// - `chrome`: 底栏状态
    /// - `input`: 原始输入文本
    /// - `cursor`: 光标字符偏移
    /// - `is_pasted`: 是否为粘贴内容
    /// - `slash_selection`: slash 命令面板的当前选中项
    ///
    /// 返回:
    /// - 可重绘的 composer source
    pub(super) fn new(
        chrome: ReplChrome,
        input: String,
        cursor: usize,
        is_pasted: bool,
        slash_selection: usize,
    ) -> Self {
        Self {
            chrome,
            input,
            cursor,
            is_pasted,
            slash_selection,
        }
    }

    /// 返回 composer 在指定终端宽度下的视觉行数。
    ///
    /// 参数:
    /// - `cols`: 终端列数
    ///
    /// 返回:
    /// - composer 所需视觉行数
    pub(super) fn height(&self, cols: usize) -> u16 {
        let layout = self.layout(cols);
        if layout.is_slash_panel() {
            return 1u16.saturating_add(layout.suggestions.len() as u16);
        }
        chrome_fixed_rows() + layout.input_rows + u16::from(!layout.suggestions.is_empty())
    }

    /// 将 composer 写入 viewport 底部并恢复输入光标位置。
    ///
    /// 参数:
    /// - `output`: 终端输出句柄
    /// - `viewport`: 当前历史与 composer 分区
    ///
    /// 返回:
    /// - 绘制是否成功
    pub(super) fn draw<W: Write>(&self, output: &mut W, viewport: &InlineViewport) -> Result<()> {
        let cols = usize::from(viewport.size().cols);
        let top = viewport.composer_top();
        let height = viewport.composer_height();
        let layout = self.layout(cols);

        // 1. 先清理整个保留区域，避免输入行数或补全提示缩短后残留旧内容
        for row_offset in 0..height {
            queue!(
                output,
                MoveTo(0, top.saturating_add(row_offset)),
                Clear(ClearType::CurrentLine)
            )?;
        }

        if layout.is_slash_panel() {
            return self.draw_slash_panel(output, top, cols, &layout);
        }

        let mut row = top;
        // 2. 顶线、输入正文、底线和状态栏均从 source 按当前宽度重新计算
        queue!(output, MoveTo(0, row), Print(chrome_rule(cols)))?;
        row = row.saturating_add(1);

        let input_start_row = row;
        for line in &layout.display_lines {
            queue!(output, MoveTo(0, row), Print(line))?;
            row = row.saturating_add(repl_line_rows_for_cols("", line, cols).max(1));
        }

        queue!(output, MoveTo(0, row), Print(chrome_rule(cols)))?;
        row = row.saturating_add(1);
        queue!(output, MoveTo(0, row), Print(self.chrome.footer_line(cols)))?;

        // 3. 历史插入会移动终端光标，最后必须把它放回可继续编辑的位置
        queue!(
            output,
            MoveTo(
                layout.cursor_col,
                input_start_row.saturating_add(layout.cursor_row_offset)
            )
        )?;
        output.flush()?;
        Ok(())
    }

    /// 根据当前列数计算输入、补全和光标布局。
    ///
    /// 参数:
    /// - `cols`: 终端列数
    ///
    /// 返回:
    /// - 当前宽度下的 composer 布局
    fn layout(&self, cols: usize) -> ComposerLayout {
        let cols = cols.max(1);
        let lines = repl_input_lines(&self.input);
        let display_lines =
            repl_visible_input_lines("", &lines, REPL_MAX_VISIBLE_INPUT_ROWS, self.is_pasted);
        let input_rows = repl_prompt_rows_for_cols("", &display_lines, cols).max(1);
        let suggestions = repl_command_suggestions(&self.input)
            .into_iter()
            .take(MAX_SLASH_PANEL_ITEMS)
            .collect();
        let (cursor_col, cursor_row_offset) = if display_lines.len() == lines.len() {
            repl_cursor_position_for_cols("", &self.input, self.cursor, cols)
        } else {
            let last_line = display_lines.last().map(String::as_str).unwrap_or_default();
            (
                (visible_width(last_line) % cols).min(u16::MAX as usize) as u16,
                input_rows.saturating_sub(1),
            )
        };
        ComposerLayout {
            display_lines,
            input_rows,
            suggestions,
            cursor_col,
            cursor_row_offset,
        }
    }

    /// 绘制 slash 命令面板，并把光标留在命令输入行。
    ///
    /// 参数:
    /// - `output`: 终端输出句柄
    /// - `top`: composer 顶部行号
    /// - `cols`: 终端列数
    /// - `layout`: 当前宽度下的 composer 布局
    ///
    /// 返回:
    /// - 绘制是否成功
    fn draw_slash_panel<W: Write>(
        &self,
        output: &mut W,
        top: u16,
        cols: usize,
        layout: &ComposerLayout,
    ) -> Result<()> {
        let input = truncate_to_width(&self.input, cols.saturating_sub(2));
        queue!(
            output,
            MoveTo(0, top),
            Print(format!("\x1b[48;5;236m> {input}\x1b[K\x1b[0m"))
        )?;
        for (index, suggestion) in layout.suggestions.iter().enumerate() {
            queue!(
                output,
                MoveTo(0, top.saturating_add(index as u16 + 1)),
                Print(format_slash_suggestion(
                    *suggestion,
                    cols,
                    index
                        == self
                            .slash_selection
                            .min(layout.suggestions.len().saturating_sub(1)),
                ))
            )?;
        }
        let before_cursor = take_chars(&self.input, self.cursor);
        let cursor_col = (2 + visible_width(&before_cursor)).min(cols.saturating_sub(1)) as u16;
        queue!(output, MoveTo(cursor_col, top))?;
        output.flush()?;
        Ok(())
    }
}

/// composer 在单一终端宽度下的计算结果。
struct ComposerLayout {
    display_lines: Vec<String>,
    input_rows: u16,
    suggestions: Vec<ReplCommandSuggestion>,
    cursor_col: u16,
    cursor_row_offset: u16,
}

impl ComposerLayout {
    /// 判断当前输入是否应展示 slash 命令面板。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 是否存在可展示的 slash 命令
    fn is_slash_panel(&self) -> bool {
        !self.suggestions.is_empty()
    }
}

/// 格式化 slash 面板的一条命令与说明。
///
/// 参数:
/// - `suggestion`: 命令建议
/// - `cols`: 终端列数
///
/// 返回:
/// - 不超过终端宽度的一行面板文本
fn format_slash_suggestion(
    suggestion: ReplCommandSuggestion,
    cols: usize,
    selected: bool,
) -> String {
    let command_width = 18usize.min(cols.saturating_sub(3));
    let description_width = cols.saturating_sub(command_width + 3);
    let description = truncate_to_width(suggestion.description, description_width);
    if selected {
        return format!(
            "\x1b[48;5;238m  \x1b[1m{:<command_width$}\x1b[0m\x1b[48;5;238m\x1b[2m{}\x1b[0m\x1b[48;5;238m\x1b[K\x1b[0m",
            suggestion.command, description
        );
    }
    format!(
        "  \x1b[1m{:<command_width$}\x1b[0m\x1b[2m{}\x1b[0m",
        suggestion.command, description
    )
}

/// 将输入或说明截断到一行可见宽度。
///
/// 参数:
/// - `value`: 原始文本
/// - `width`: 最大显示宽度
///
/// 返回:
/// - 不超过最大宽度的文本
fn truncate_to_width(value: &str, width: usize) -> String {
    if visible_width(value) <= width {
        return value.to_string();
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    let mut output = String::new();
    let mut used = 0usize;
    for ch in value.chars() {
        let char_width = visible_width(&ch.to_string());
        if used.saturating_add(char_width) > width - 3 {
            break;
        }
        output.push(ch);
        used = used.saturating_add(char_width);
    }
    output.push_str("...");
    output
}

#[cfg(test)]
mod tests {
    use super::ComposerFrame;
    use crate::agent::AgentMode;
    use crate::cli::repl_chrome::ReplChrome;
    use crate::cli::repl_runtime::viewport::{InlineViewport, TerminalSize};

    /// 验证 composer 在固定 viewport 内写入底部，并将光标放回输入行。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn draws_at_viewport_bottom_and_restores_input_cursor() {
        let chrome = ReplChrome {
            mode: AgentMode::Yolo,
            context_ratio: 0.0,
            context_window_tokens: 120_000,
            model: "gpt".to_string(),
            thinking: "auto".to_string(),
            directory: "/workspace".to_string(),
            git_branch: None,
        };
        let frame = ComposerFrame::new(chrome, "hello".to_string(), 5, false, 0);
        let mut viewport = InlineViewport::new();
        viewport.update(TerminalSize { cols: 40, rows: 12 }, frame.height(40), 8);
        let mut output = Vec::new();

        frame.draw(&mut output, &viewport).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\x1b[9;1H"));
        assert!(output.contains("\x1b[10;6H"));
    }

    /// 验证 slash 命令面板隐藏常规状态栏并展示命令说明。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn slash_panel_replaces_footer_with_command_descriptions() {
        let chrome = ReplChrome {
            mode: AgentMode::Yolo,
            context_ratio: 0.0,
            context_window_tokens: 120_000,
            model: "gpt".to_string(),
            thinking: "auto".to_string(),
            directory: "/workspace".to_string(),
            git_branch: None,
        };
        let frame = ComposerFrame::new(chrome, "/".to_string(), 1, false, 0);
        let mut viewport = InlineViewport::new();
        viewport.update(TerminalSize { cols: 72, rows: 24 }, frame.height(72), 4);
        let mut output = Vec::new();

        frame.draw(&mut output, &viewport).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("/model"));
        assert!(!output.contains("120k"));
    }
}
