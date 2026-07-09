use crate::render::style::{RESET, TABLE_BORDER_STYLE};
use crossterm::terminal;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

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
/// `math_source` 在布局确定最终列宽后用于按列宽重渲染公式图片。
#[derive(Clone)]
pub(crate) struct CellContent {
    pub lines: Vec<String>,
    pub width: usize,
    pub is_image: bool,
    pub math_source: Option<String>,
}

impl CellContent {
    /// 从单行带 ANSI 样式的文本构建。
    pub fn from_inline(text: String) -> Self {
        let width = visible_width(&text);
        Self {
            lines: vec![text],
            width,
            is_image: false,
            math_source: None,
        }
    }

    /// 空单元格。
    pub fn empty() -> Self {
        Self {
            lines: vec![String::new()],
            width: 0,
            is_image: false,
            math_source: None,
        }
    }

    /// 构造图片单元格。
    ///
    /// 参数:
    /// - `lines`: 图片协议或半块文本行
    /// - `width`: 声明的终端列宽
    /// - `math_source`: 可选公式源码，用于按最终列宽重渲染
    ///
    /// 返回:
    /// - 图片单元格内容
    pub fn from_image(lines: Vec<String>, width: usize, math_source: Option<String>) -> Self {
        Self {
            lines,
            width: width.max(1),
            is_image: true,
            math_source,
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
    let mut rows = lines
        .iter()
        .filter(|line| !is_table_separator(line))
        .map(|line| parse_table_row(line, render_cell))
        .collect::<Vec<_>>();
    let widths = compute_table_widths(&rows);
    // 1. 列宽确定后，按最终列宽重渲染公式图片，避免协议占位与列宽不一致
    refit_math_image_cells(&mut rows, &widths);

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

/// 按最终列宽重渲染带公式源的图片单元格。
///
/// 参数:
/// - `rows`: 表格行
/// - `widths`: 最终列宽
fn refit_math_image_cells(rows: &mut [Vec<CellContent>], widths: &[usize]) {
    for row in rows.iter_mut() {
        for (index, cell) in row.iter_mut().enumerate() {
            let Some(target_width) = widths.get(index).copied() else {
                continue;
            };
            let Some(encoded) = cell.math_source.clone() else {
                continue;
            };
            if !cell.is_image || cell.width <= target_width {
                continue;
            }
            // 1. 列被压缩时按最终列宽重编码 c/r，避免图宽大于列宽
            let (source, mixed) = crate::render::asset_block::decode_table_math_source(&encoded);
            let mut refitted =
                crate::render::asset_block::render_inline_math_table_cell(
                    &source,
                    target_width,
                    mixed,
                );
            refitted.math_source = Some(encoded);
            *cell = refitted;
        }
    }
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
    split_table_cells(line)
        .into_iter()
        .map(|cell| render_cell(&cell))
        .collect()
}

/// 解析表格对齐标记。
pub(crate) fn parse_table_alignments(line: &str) -> Vec<TableAlign> {
    split_table_cells(line)
        .into_iter()
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

/// 按表格分隔符拆分单元格，忽略公式/代码内的 `|` 与转义 `\|`。
///
/// 参数:
/// - `line`: 原始表格行
///
/// 返回:
/// - 单元格文本列表（已 trim）
pub(crate) fn split_table_cells(line: &str) -> Vec<String> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = if line.ends_with('|') && !line.ends_with("\\|") {
        &line[..line.len().saturating_sub(1)]
    } else {
        line
    };
    let chars = line.chars().collect::<Vec<_>>();
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut index = 0usize;
    let mut escaped = false;
    let mut in_inline_math = false;
    let mut in_display_math = false;
    let mut in_code = false;
    while index < chars.len() {
        let ch = chars[index];
        // 1. 反斜杠转义：下一字符原样写入，不作为分隔符
        if escaped {
            current.push(ch);
            escaped = false;
            index += 1;
            continue;
        }
        if ch == '\\' {
            current.push(ch);
            escaped = true;
            index += 1;
            continue;
        }
        // 2. 显示公式 $$...$$
        if ch == '$' && chars.get(index + 1) == Some(&'$') {
            in_display_math = !in_display_math;
            if !in_display_math {
                in_inline_math = false;
            }
            current.push('$');
            current.push('$');
            index += 2;
            continue;
        }
        // 3. 行内公式 $...$
        if ch == '$' && !in_display_math {
            in_inline_math = !in_inline_math;
            current.push(ch);
            index += 1;
            continue;
        }
        // 4. 行内代码 `...`
        if ch == '`' && !in_inline_math && !in_display_math {
            in_code = !in_code;
            current.push(ch);
            index += 1;
            continue;
        }
        // 5. 仅在公式/代码外的 | 才是列分隔
        if ch == '|' && !in_inline_math && !in_display_math && !in_code {
            cells.push(current.trim().to_string());
            current.clear();
            index += 1;
            continue;
        }
        current.push(ch);
        index += 1;
    }
    cells.push(current.trim().to_string());
    cells
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
    bounded_table_widths(rows, widths)
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
            output.push(' ');
            if is_image {
                // 1. 图片协议载荷宽度为 0，需按声明列宽推进光标
                // 2. 占位空行用空格填满列宽，避免后续列左移
                push_image_cell_line(&mut output, &line, cell.map(|c| c.width).unwrap_or(0), *width);
            } else {
                let content_width = visible_width(&line);
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

/// 写入图片单元格的一行内容。
///
/// 参数:
/// - `output`: 输出缓冲
/// - `line`: 当前行文本（首行为协议载荷，其余行为空占位）
/// - `image_width`: 图片声明的终端列宽
/// - `column_width`: 表格最终列宽
fn push_image_cell_line(output: &mut String, line: &str, image_width: usize, column_width: usize) {
    if line.is_empty() {
        // 垂直占位行：纯空格保留列宽
        output.push_str(&" ".repeat(column_width));
        return;
    }
    if is_graphics_protocol_line(line) {
        // 不填充列宽、不做 CSI 前进，只输出协议载荷
        let _ = (image_width, column_width);
        output.push_str(line);
        return;
    }
    // 半块等字符型内容会自然推进光标
    let content_width = visible_width(line);
    output.push_str(line);
    output.push_str(&" ".repeat(column_width.saturating_sub(content_width)));
}

/// 判断是否为终端图形协议载荷（Kitty / iTerm2 / Sixel）。
///
/// 参数:
/// - `line`: 单元格行文本
///
/// 返回:
/// - 是否为图形协议序列
fn is_graphics_protocol_line(line: &str) -> bool {
    line.starts_with("\x1b_G") || line.starts_with("\x1b]1337;") || line.starts_with("\x1bP")
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
    let mut visible_segment = String::new();
    let mut escape = false;
    let mut csi = false;
    for ch in text.chars() {
        if ch == '\x1b' {
            width += text_display_width(&visible_segment);
            visible_segment.clear();
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
        } else {
            visible_segment.push(ch);
        }
    }
    width + text_display_width(&visible_segment)
}

/// 将表格宽度限制在终端宽度内。
///
/// 优先压缩纯文本列；含图片的列尽量不低于图片声明宽度，避免协议占位与列宽错位。
///
/// 参数:
/// - `rows`: 已渲染的表格行
/// - `widths`: 初步列宽
///
/// 返回:
/// - 适配终端后的列宽
fn bounded_table_widths(rows: &[Vec<CellContent>], mut widths: Vec<usize>) -> Vec<usize> {
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
    // 1. 计算每列图片内容的最小保留宽度
    let image_mins = (0..widths.len())
        .map(|index| {
            rows.iter()
                .filter_map(|row| row.get(index))
                .filter(|cell| cell.is_image)
                .map(|cell| cell.width.max(1))
                .max()
                .unwrap_or(1)
        })
        .collect::<Vec<_>>();
    while widths.iter().sum::<usize>() > available {
        // 2. 优先缩小仍高于图片最小宽度（或纯文本）的最宽列
        let Some((index, width)) = widths
            .iter()
            .enumerate()
            .filter(|(index, width)| **width > image_mins[*index])
            .max_by_key(|(_, width)| **width)
            .map(|(index, width)| (index, *width))
        else {
            // 3. 全部列已贴图片下限仍超宽时，再允许压缩图片列
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
            continue;
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
    let mut active_style = String::new();
    let mut index = 0usize;
    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or_default();
        if ch == '\x1b' {
            let (sequence, next_index) = collect_ansi_sequence_at(text, index);
            update_active_style(&sequence, &mut active_style);
            current.push_str(&sequence);
            index = next_index;
            continue;
        }
        let grapheme = text[index..].graphemes(true).next().unwrap_or("");
        let grapheme_width = text_display_width(grapheme);
        if current_width > 0 && current_width + grapheme_width > width {
            if !active_style.is_empty() {
                current.push_str(RESET);
            }
            lines.push(current);
            current = active_style.clone();
            current_width = 0;
        }
        current.push_str(grapheme);
        current_width += grapheme_width;
        index += grapheme.len();
    }
    lines.push(current);
    lines
}

/// 从指定字节位置收集完整 ANSI 转义序列。
///
/// 参数:
/// - `text`: 原始文本
/// - `start`: ANSI 转义序列起始字节位置
///
/// 返回:
/// - 完整 ANSI 转义序列和下一个字节位置
fn collect_ansi_sequence_at(text: &str, start: usize) -> (String, usize) {
    let mut sequence = String::new();
    let mut chars = text[start..].char_indices();
    let Some((_, first)) = chars.next() else {
        return (sequence, start);
    };
    sequence.push(first);
    let mut next_index = start + first.len_utf8();
    let Some((offset, next)) = chars.next() else {
        return (sequence, next_index);
    };
    sequence.push(next);
    next_index = start + offset + next.len_utf8();
    if next == '[' {
        for (offset, ch) in chars.by_ref() {
            sequence.push(ch);
            next_index = start + offset + ch.len_utf8();
            if (ch as u32) >= 0x40 && (ch as u32) <= 0x7e {
                break;
            }
        }
    } else if next != '\\' {
        for (offset, ch) in chars.by_ref() {
            sequence.push(ch);
            next_index = start + offset + ch.len_utf8();
            if ch == '\\' {
                break;
            }
        }
    }
    (sequence, next_index)
}

/// 根据 SGR 转义序列更新当前活动样式。
///
/// 参数:
/// - `sequence`: ANSI 转义序列
/// - `active_style`: 当前活动样式缓冲
fn update_active_style(sequence: &str, active_style: &mut String) {
    if !sequence.starts_with("\x1b[") || !sequence.ends_with('m') {
        return;
    }
    if sequence == RESET || sequence == "\x1b[m" {
        active_style.clear();
    } else {
        active_style.push_str(sequence);
    }
}

/// 计算一段可见文本的终端显示宽度。
///
/// 参数:
/// - `text`: 不含 ANSI 转义序列的文本
///
/// 返回:
/// - 文本在终端中的显示列宽
fn text_display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
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
    use crate::render::style::INLINE_CODE_STYLE;

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
        assert_eq!(
            middle_border_count, 2,
            "expected 2 middle borders, got: {output}"
        );
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

    #[test]
    fn visible_width_counts_emoji_check_mark_as_wide() {
        assert_eq!(visible_width("\u{2705} ok"), 5);
        assert_eq!(visible_width("\u{1f5a5}\u{fe0f} system"), 9);
    }

    #[test]
    fn table_rows_keep_same_width_with_emoji_status_cells() {
        let ok_mark = "\u{2705}";
        let desktop = "\u{1f5a5}\u{fe0f}";
        let green_circle = "\u{1f7e2}";
        let output = render_table(
            &[
                "| 操作 | 结果 |".to_string(),
                "|---|---|".to_string(),
                format!("| background_command | {ok_mark} PID 832048 |"),
                format!("| background_command | {ok_mark} stopped |"),
                format!("| {desktop} 系统 | {green_circle} 正常 |"),
            ],
            render_table_cell_content,
        );
        let line_widths = output.lines().map(visible_width).collect::<Vec<_>>();
        let Some(first_width) = line_widths.first().copied() else {
            panic!("expected rendered table output");
        };
        assert!(
            line_widths.iter().all(|width| *width == first_width),
            "table lines should have identical widths: {line_widths:?}\n{output}"
        );
    }

    #[test]
    fn table_rows_keep_same_width_with_system_status_emojis() {
        let desktop = "\u{1f5a5}\u{fe0f}";
        let package = "\u{1f4e6}";
        let globe = "\u{1f310}";
        let gamepad = "\u{1f3ae}";
        let music = "\u{1f3b5}";
        let plug = "\u{1f50c}";
        let green_circle = "\u{1f7e2}";
        let output = render_table(
            &[
                "| 项目 | 状态 | 说明 |".to_string(),
                "|---|---|---|".to_string(),
                format!("| {desktop} 系统 | {green_circle} 正常 | Arch Linux, kernel 6.x |"),
                format!("| {package} 包管理 | {green_circle} 正常 | pacman + yay |"),
                format!("| {globe} 网络 | {green_circle} 在线 | eth0 已连接 |"),
                format!("| {gamepad} 显卡 | {green_circle} 驱动正常 | nvidia-open |"),
                format!("| {music} 音频 | {green_circle} 工作中 | pipewire |"),
                format!("| {plug} 输入法 | {green_circle} 可用 | fcitx5 |"),
            ],
            render_table_cell_content,
        );
        let line_widths = output.lines().map(visible_width).collect::<Vec<_>>();
        let Some(first_width) = line_widths.first().copied() else {
            panic!("expected rendered table output");
        };
        assert!(
            line_widths.iter().all(|width| *width == first_width),
            "table lines should have identical widths: {line_widths:?}\n{output}"
        );
    }

    #[test]
    fn wrap_ansi_text_preserves_inline_code_style_across_lines() {
        let text = format!("{INLINE_CODE_STYLE}sudo pacman -S neovim{RESET}");

        let lines = wrap_ansi_text(&text, 12);

        assert!(lines.len() > 1);
        assert!(lines[0].contains(INLINE_CODE_STYLE));
        assert!(lines[0].ends_with(RESET));
        assert!(lines[1].starts_with(INLINE_CODE_STYLE));
        assert!(lines.last().unwrap().ends_with(RESET));
        assert_eq!(
            strip_ansi_for_test(&lines.join("")),
            "sudo pacman -S neovim"
        );
    }

    #[test]
    fn image_cell_placeholder_lines_reserve_column_width() {
        let mut output = String::new();
        push_image_cell_line(&mut output, "", 12, 10);
        assert_eq!(output, " ".repeat(10));

        let mut output = String::new();
        // 半块文本按可见宽度补齐，而不是光标前进
        push_image_cell_line(&mut output, "abc", 3, 6);
        assert_eq!(output, "abc   ");
    }

    #[test]
    fn graphics_protocol_line_emits_payload_only() {
        let mut output = String::new();
        let protocol = "\x1b_Gf=100,a=T,q=2,c=8,r=2;abc\x1b\\";
        push_image_cell_line(&mut output, protocol, 8, 10);
        assert_eq!(output, protocol);
        assert!(!output.contains("\x1b[8C"));
    }

    #[test]
    fn pure_math_table_cells_keep_border_structure() {
        let output = render_table(
            &[
                "| 名称 | 公式 | 说明 |".to_string(),
                "|---|---|---|".to_string(),
                "| 勾股 | $a^2+b^2=c^2$ | 直角 |".to_string(),
                "| 欧拉 | $e^{i\\\\pi}+1=0$ | 恒等式 |".to_string(),
            ],
            render_table_cell_content,
        );
        assert!(output.contains('┌'));
        assert!(output.contains('│'));
        assert!(output.matches('├').count() >= 2);
        // 边框行可见宽度应一致
        let border_widths = output
            .lines()
            .filter(|line| line.contains('─'))
            .map(visible_width)
            .collect::<Vec<_>>();
        if let Some(first) = border_widths.first() {
            assert!(
                border_widths.iter().all(|w| w == first),
                "border widths differ: {border_widths:?}"
            );
        }
    }

    #[test]
    fn split_table_cells_keeps_pipes_inside_math() {
        let cells = split_table_cells(
            r#"| 概率 | 全概率 | $P(B)=\sum_i P(A_i)P(B|A_i)$ | 通过 |"#,
        );
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0], "概率");
        assert_eq!(cells[1], "全概率");
        assert_eq!(cells[2], r"$P(B)=\sum_i P(A_i)P(B|A_i)$");
        assert_eq!(cells[3], "通过");
    }

    #[test]
    fn split_table_cells_keeps_escaped_pipes() {
        let cells = split_table_cells(r#"| a \| b | c |"#);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0], r"a \| b");
        assert_eq!(cells[1], "c");
    }

    #[test]
    fn math_with_pipe_table_keeps_column_count() {
        let output = render_table(
            &[
                "| 名称 | 公式 | 状态 |".to_string(),
                "|---|---|---|".to_string(),
                r#"| 全概率 | $P(B)=\sum_i P(A_i)P(B|A_i)$ | ok |"#.to_string(),
            ],
            render_table_cell_content,
        );
        // 三列表格应有 4 条竖线位置（含左右边框）
        let data_line = output
            .lines()
            .find(|line| line.contains('│') && !line.contains('─'))
            .expect("data line");
        let bars = data_line.matches('│').count();
        assert!(
            bars >= 4,
            "expected at least 4 vertical bars for 3 columns, got {bars}: {data_line}"
        );
        assert!(!output.contains(r"$P(B)=\sum_i"));
    }

    /// 去除 ANSI 转义序列，方便断言表格可见文本。
    ///
    /// 参数:
    /// - `text`: 原始终端文本
    ///
    /// 返回:
    /// - 去除样式后的文本
    fn strip_ansi_for_test(text: &str) -> String {
        let mut output = String::new();
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
                } else if ch == '\\' || ch == 'm' {
                    escape = false;
                }
            } else {
                output.push(ch);
            }
        }
        output
    }
}
