use crate::render::style::{RESET, TABLE_BORDER_STYLE};
use crossterm::terminal;

const TOP_LEFT: char = '┌';
const TOP_MID: char = '┬';
const TOP_RIGHT: char = '┐';
const MID_LEFT: char = '├';
const MID_MID: char = '┼';
const MID_RIGHT: char = '┤';
const BOTTOM_LEFT: char = '└';
const BOTTOM_MID: char = '┴';
const BOTTOM_RIGHT: char = '┘';
const HORIZONTAL: char = '─';
const VERTICAL: char = '│';

#[derive(Clone, Copy)]
pub(crate) enum TableAlign {
    Left,
    Center,
    Right,
}

/// 已渲染的表格单元格内容，携带显示宽度。
///
/// 对于含终端图片协议（如 Sixel）的单元格，`visible_width` 无法正确计算宽度，
/// 因此由渲染方直接提供 `width`。
#[derive(Clone)]
pub(crate) struct CellContent {
    pub lines: Vec<String>,
    pub width: usize,
    pub is_image: bool,
}

impl CellContent {
    /// 从单行带 ANSI 样式的文本构建。
    pub fn from_inline(text: String) -> Self {
        let width = visible_width(&text);
        Self {
            lines: vec![text],
            width,
            is_image: false,
        }
    }

    /// 空单元格。
    pub fn empty() -> Self {
        Self {
            lines: vec![String::new()],
            width: 0,
            is_image: false,
        }
    }
}

/// 渲染完整 Markdown 表格。
///
/// 参数:
/// - `lines`: 表格原始行
/// - `render_cell`: 单元格渲染函数，返回 `CellContent`
///
/// 返回:
/// - 带细实线边框的终端表格文本
pub(crate) fn render_table<F>(lines: &[String], render_cell: F) -> String
where
    F: Fn(&str) -> CellContent + Copy,
{
    let alignments = lines
        .get(1)
        .filter(|line| is_table_separator(line))
        .map(|line| parse_table_alignments(line))
        .unwrap_or_default();
    let rows = lines
        .iter()
        .filter(|line| !is_table_separator(line))
        .map(|line| parse_table_row(line, render_cell))
        .collect::<Vec<_>>();
    let widths = compute_table_widths(&rows);

    let mut output = String::new();
    output.push_str(&top_border(&widths));
    for (row_index, row) in rows.iter().enumerate() {
        output.push_str(&render_table_row(row, &widths, &alignments, row_index == 0));
        if row_index + 1 < rows.len() {
            output.push_str(&middle_border(&widths));
        }
    }
    output.push_str(&bottom_border(&widths));
    output
}

/// 判断一行是否为 Markdown 表格分隔行。
pub(crate) fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim().trim_matches('|').trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '-' | ':' | '|' | ' '))
        && trimmed.contains('-')
}

/// 判断一行是否像 Markdown 表格行。
pub(crate) fn looks_like_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.matches('|').count() >= 2
}

/// 解析表格数据行。
///
/// 参数:
/// - `line`: 原始表格行
/// - `render_cell`: 单元格渲染函数
///
/// 返回:
/// - 已渲染的单元格内容列表
pub(crate) fn parse_table_row<F>(line: &str, render_cell: F) -> Vec<CellContent>
where
    F: Fn(&str) -> CellContent,
{
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| render_cell(cell.trim()))
        .collect()
}

/// 解析表格对齐标记。
pub(crate) fn parse_table_alignments(line: &str) -> Vec<TableAlign> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| {
            let cell = cell.trim();
            match (cell.starts_with(':'), cell.ends_with(':')) {
                (true, true) => TableAlign::Center,
                (false, true) => TableAlign::Right,
                _ => TableAlign::Left,
            }
        })
        .collect()
}

/// 根据列数量返回最小列宽，防止短内容列过窄。
pub(crate) fn readable_table_min_width(cols: usize) -> usize {
    match cols {
        0 => 0,
        1 => 16,
        2 => 14,
        3 | 4 => 10,
        _ => 8,
    }
}

/// 根据所有已渲染行计算表格列宽。
pub(crate) fn compute_table_widths(rows: &[Vec<CellContent>]) -> Vec<usize> {
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0usize; cols];
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.width);
        }
    }
    let min_width = readable_table_min_width(cols);
    for width in &mut widths {
        *width = (*width).max(min_width);
    }
    bounded_table_widths(widths)
}

/// 渲染单个表格数据行。
///
/// 支持多行单元格（如 Sixel 图片占多行），其余单元格自动补齐空行。
/// 每个单元格的内容会按列宽自动换行。
pub(crate) fn render_table_row(
    row: &[CellContent],
    widths: &[usize],
    alignments: &[TableAlign],
    header: bool,
) -> String {
    let wrapped: Vec<Vec<String>> = widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let cell = row.get(index);
            let is_image = cell.map(|c| c.is_image).unwrap_or(false);
            let cell_lines = cell.map(|c| c.lines.as_slice()).unwrap_or(&[]);
            let mut all_lines = Vec::new();
            for line in cell_lines {
                if is_image {
                    all_lines.push(line.clone());
                } else {
                    all_lines.extend(wrap_ansi_text(line, *width));
                }
            }
            if all_lines.is_empty() {
                all_lines.push(String::new());
            }
            all_lines
        })
        .collect();
    let row_height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
    let mut output = String::new();
    for line_index in 0..row_height {
        push_vertical(&mut output);
        for (index, width) in widths.iter().enumerate() {
            let cell = row.get(index);
            let is_image = cell.map(|c| c.is_image).unwrap_or(false);
            let line = wrapped
                .get(index)
                .and_then(|lines| lines.get(line_index))
                .map(String::as_str)
                .unwrap_or("");
            let line = if header && !line.is_empty() {
                format!("\x1b[1m{line}\x1b[0m")
            } else {
                line.to_string()
            };
            let content_width = if is_image {
                cell.map(|c| c.width).unwrap_or(0)
            } else {
                visible_width(&line)
            };
            output.push(' ');
            if is_image {
                output.push_str(&line);
                let advance = content_width.min(*width);
                output.push_str(&format!("\x1b[{advance}C"));
                let padding = width.saturating_sub(content_width);
                output.push_str(&" ".repeat(padding));
            } else {
                output.push_str(&aligned_cell_with_width(
                    &line,
                    content_width,
                    *width,
                    alignments.get(index).copied().unwrap_or(TableAlign::Left),
                ));
            }
            output.push(' ');
            push_vertical(&mut output);
        }
        output.push('\n');
    }
    output
}

/// 渲染表格顶部边框。
pub(crate) fn top_border(widths: &[usize]) -> String {
    table_border(widths, TOP_LEFT, TOP_MID, TOP_RIGHT)
}

/// 渲染表格中部分隔边框。
pub(crate) fn middle_border(widths: &[usize]) -> String {
    table_border(widths, MID_LEFT, MID_MID, MID_RIGHT)
}

/// 渲染表格底部边框。
pub(crate) fn bottom_border(widths: &[usize]) -> String {
    table_border(widths, BOTTOM_LEFT, BOTTOM_MID, BOTTOM_RIGHT)
}

/// 计算去除 ANSI 转义后的显示宽度。
///
/// 处理 CSI 序列（`\x1b[...`，以 0x40-0x7e 结尾）、SGR（`\x1bm`）
/// 和 ST 终止符（`\x1b\\`，用于 Kitty/Sixel/iTerm 协议）。
pub(crate) fn visible_width(text: &str) -> usize {
    let mut width = 0;
    let mut escape = false;
    let mut csi = false;
    for ch in text.chars() {
        if ch == '\x1b' {
            escape = true;
            csi = false;
        } else if escape {
            if csi {
                if (ch as u32) >= 0x40 && (ch as u32) <= 0x7e {
                    escape = false;
                }
            } else if ch == '[' {
                csi = true;
            } else if ch == '\\' {
                escape = false;
            } else if ch == 'm' {
                escape = false;
            }
        } else if (ch as u32) >= 0x2e80 {
            width += 2;
        } else {
            width += 1;
        }
    }
    width
}

/// 将表格宽度限制在终端宽度内。
fn bounded_table_widths(mut widths: Vec<usize>) -> Vec<usize> {
    if widths.is_empty() {
        return widths;
    }
    let terminal_width = terminal::size()
        .map(|(width, _)| usize::from(width))
        .unwrap_or(100)
        .saturating_sub(1)
        .max(20);
    let border_overhead = widths.len().saturating_mul(3).saturating_add(1);
    let available = terminal_width
        .saturating_sub(border_overhead)
        .max(widths.len());
    while widths.iter().sum::<usize>() > available {
        let Some((index, width)) = widths
            .iter()
            .enumerate()
            .max_by_key(|(_, width)| **width)
            .map(|(index, width)| (index, *width))
        else {
            break;
        };
        if width <= 1 {
            break;
        }
        widths[index] -= 1;
    }
    widths
}

/// 将含 ANSI 转义的文本按宽度换行。
///
/// 参数:
/// - `text`: 原始文本
/// - `width`: 目标宽度
///
/// 返回:
/// - 换行后的文本行
fn wrap_ansi_text(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            current.push(ch);
            for next in chars.by_ref() {
                current.push(next);
                if next == 'm' {
                    break;
                }
            }
            continue;
        }
        let ch_width = char_display_width(ch);
        if current_width > 0 && current_width + ch_width > width {
            lines.push(current);
            current = String::new();
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;
    }
    lines.push(current);
    lines
}

/// 计算单个字符的显示宽度。
fn char_display_width(ch: char) -> usize {
    if ch.is_ascii() {
        1
    } else {
        2
    }
}

/// 按对齐方式和已知内容宽度补齐单元格。
///
/// 参数:
/// - `cell`: 已渲染的单行内容
/// - `content_width`: 内容的实际显示宽度
/// - `column_width`: 列的目标宽度
/// - `align`: 对齐方式
fn aligned_cell_with_width(
    cell: &str,
    content_width: usize,
    column_width: usize,
    align: TableAlign,
) -> String {
    let padding = column_width.saturating_sub(content_width);
    match align {
        TableAlign::Left => format!("{cell}{}", " ".repeat(padding)),
        TableAlign::Right => format!("{}{cell}", " ".repeat(padding)),
        TableAlign::Center => {
            let left = padding / 2;
            let right = padding - left;
            format!("{}{cell}{}", " ".repeat(left), " ".repeat(right))
        }
    }
}

/// 渲染通用表格边框。
fn table_border(widths: &[usize], left: char, mid: char, right: char) -> String {
    let mut output = String::new();
    output.push_str(TABLE_BORDER_STYLE);
    output.push(left);
    for (index, width) in widths.iter().enumerate() {
        output.push_str(&HORIZONTAL.to_string().repeat(width + 2));
        output.push(if index + 1 == widths.len() {
            right
        } else {
            mid
        });
    }
    output.push_str(RESET);
    output.push('\n');
    output
}

/// 写入表格竖线。
fn push_vertical(output: &mut String) {
    output.push_str(TABLE_BORDER_STYLE);
    output.push(VERTICAL);
    output.push_str(RESET);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::markdown::render_table_cell_content;

    #[test]
    fn table_uses_thin_box_borders() {
        let output = render_table(
            &[
                "| a | b |".to_string(),
                "| - | - |".to_string(),
                "| 1 | 2 |".to_string(),
            ],
            render_table_cell_content,
        );
        assert!(output.contains('┌'));
        assert!(output.contains('┬'));
        assert!(output.contains('├'));
        assert!(output.contains('┼'));
        assert!(output.contains('└'));
        assert!(output.contains('│'));
        assert!(output.contains('─'));
        assert!(!output.contains('+'));
    }

    #[test]
    fn table_draws_middle_borders_between_all_rows() {
        let output = render_table(
            &[
                "| a | b |".to_string(),
                "| - | - |".to_string(),
                "| 1 | 2 |".to_string(),
                "| 3 | 4 |".to_string(),
            ],
            render_table_cell_content,
        );
        let middle_border_count = output.matches('├').count();
        assert_eq!(middle_border_count, 2, "expected 2 middle borders, got: {output}");
    }

    #[test]
    fn readable_table_min_width_returns_expected_values() {
        assert_eq!(readable_table_min_width(0), 0);
        assert_eq!(readable_table_min_width(1), 16);
        assert_eq!(readable_table_min_width(2), 14);
        assert_eq!(readable_table_min_width(3), 10);
        assert_eq!(readable_table_min_width(4), 10);
        assert_eq!(readable_table_min_width(5), 8);
        assert_eq!(readable_table_min_width(10), 8);
    }

    #[test]
    fn short_tables_use_content_width() {
        let output = render_table(
            &[
                "| 项目 | 内容 |".to_string(),
                "|---|---|".to_string(),
                "| 名字 | 未有 / Miyu |".to_string(),
                "| 年龄 | 18 |".to_string(),
            ],
            render_table_cell_content,
        );
        let terminal_width = terminal::size()
            .map(|(width, _)| usize::from(width))
            .unwrap_or(100);
        let widest = output.lines().map(visible_width).max().unwrap_or(0);
        assert!(widest < terminal_width / 2, "table too wide: {widest}");
    }

    #[test]
    fn wraps_wide_table_cells_to_terminal_width() {
        let output = render_table(
            &[
                "| 项目 | 内容 |".to_string(),
                "|---|---|".to_string(),
                format!("| 很长 | {} |", "这是一段非常长的内容".repeat(20)),
            ],
            render_table_cell_content,
        );
        let terminal_width = terminal::size()
            .map(|(width, _)| usize::from(width))
            .unwrap_or(100);
        for line in output.lines() {
            assert!(
                visible_width(line) < terminal_width,
                "line too wide: {line}"
            );
        }
        assert!(output.lines().count() > 5);
    }
}
