use super::repl_chrome::{chrome_fixed_rows, chrome_rule, ReplChrome};
use super::*;

/// 渲染极简 REPL 输入框与底栏。
///
/// 布局：
/// 1. 顶部分隔线
/// 2. 输入文本（无模式前缀）
/// 3. 底部分隔线
/// 4. 状态行：上下文占用 | 模型 · 思考等级
/// 5. 模式标签
/// 6. 可选斜杠补全提示
///
/// 参数:
/// - `stdout`: 终端输出
/// - `input_row`: 输入区起始行（可上移）
/// - `rendered_rows`: 上次渲染占用行数
/// - `chrome`: 底栏状态
/// - `input`: 当前输入
/// - `cursor`: 光标字符偏移
/// - `is_pasted`: 是否粘贴内容
///
/// 返回:
/// - 渲染是否成功
pub(super) fn render_repl_input(
    stdout: &mut io::Stdout,
    input_row: &mut u16,
    rendered_rows: &mut u16,
    chrome: &ReplChrome,
    input: &str,
    cursor: usize,
    is_pasted: bool,
) -> Result<()> {
    let cols = terminal_cols();
    let suggestions = repl_command_suggestions(input);
    let lines = repl_input_lines(input);
    // 1. 输入区无模式前缀，保持极简
    let plain_prefix = "";
    let display_lines = repl_visible_input_lines(
        plain_prefix,
        &lines,
        REPL_MAX_VISIBLE_INPUT_ROWS,
        is_pasted,
    );
    let input_rows = repl_prompt_rows(plain_prefix, &display_lines).max(1);
    let current_rows = chrome_fixed_rows()
        + input_rows
        + u16::from(!suggestions.is_empty());
    let rows_to_clear = (*rendered_rows).max(current_rows).max(1);
    ensure_repl_space(stdout, input_row, rows_to_clear)?;

    // 2. 清空旧区域
    for row_offset in 0..rows_to_clear {
        queue!(
            stdout,
            MoveTo(0, (*input_row).saturating_add(row_offset)),
            Clear(ClearType::CurrentLine)
        )?;
    }

    let mut row = *input_row;
    // 3. 顶线
    queue!(
        stdout,
        MoveTo(0, row),
        Print(chrome_rule(cols))
    )?;
    row = row.saturating_add(1);

    // 4. 输入正文
    let input_start_row = row;
    for (index, line) in display_lines.iter().enumerate() {
        queue!(stdout, MoveTo(0, row), Print(line))?;
        let used = if index == 0 {
            repl_line_rows(plain_prefix, line)
        } else {
            repl_line_rows("", line)
        };
        row = row.saturating_add(used.max(1));
    }
    if display_lines.is_empty() {
        row = row.saturating_add(1);
    }

    // 5. 底线
    queue!(
        stdout,
        MoveTo(0, row),
        Print(chrome_rule(cols))
    )?;
    row = row.saturating_add(1);

    // 6. 状态栏：yolo/plan 在上下文左侧，右侧为模型 · 思考等级
    queue!(stdout, MoveTo(0, row), Print(chrome.footer_line(cols)))?;
    row = row.saturating_add(1);

    // 7. 斜杠补全
    if !suggestions.is_empty() {
        queue!(
            stdout,
            MoveTo(0, row),
            Print(format!("\x1b[2m{}\x1b[0m", suggestions.join("  ")))
        )?;
    }

    // 9. 光标回到输入区
    let (cursor_col, cursor_row_offset) = if display_lines.len() == lines.len() {
        repl_cursor_position(plain_prefix, input, cursor)
    } else {
        let last_line = display_lines.last().map(String::as_str).unwrap_or_default();
        let col = (visible_width(last_line) % cols.max(1)) as u16;
        (
            col,
            repl_prompt_rows(plain_prefix, &display_lines).saturating_sub(1),
        )
    };
    queue!(
        stdout,
        MoveTo(
            cursor_col,
            input_start_row.saturating_add(cursor_row_offset)
        )
    )?;
    stdout.flush()?;
    *rendered_rows = current_rows;
    Ok(())
}

pub(super) fn repl_visible_input_lines(
    prefix: &str,
    lines: &[String],
    max_rows: u16,
    is_pasted: bool,
) -> Vec<String> {
    let total_rows = repl_prompt_rows(prefix, lines);
    if total_rows <= max_rows || lines.len() <= 2 || !is_pasted {
        return lines.to_vec();
    }

    let omitted_lines = lines.len().saturating_sub(2);
    let omitted = if is_zh() {
        format!("... 已隐藏 {omitted_lines} 行粘贴内容 ...")
    } else {
        format!("... {omitted_lines} pasted lines hidden ...")
    };
    vec![lines[0].clone(), omitted, lines[lines.len() - 1].clone()]
}

fn ensure_repl_space(stdout: &mut io::Stdout, input_row: &mut u16, needed_rows: u16) -> Result<()> {
    let (_, term_rows) = terminal::size().unwrap_or((80, 24));
    let term_rows = term_rows.max(1);
    if (*input_row).saturating_add(needed_rows) < term_rows {
        return Ok(());
    }
    let overflow = (*input_row)
        .saturating_add(needed_rows)
        .saturating_sub(term_rows.saturating_sub(1));
    queue!(stdout, MoveTo(0, term_rows.saturating_sub(1)))?;
    for _ in 0..overflow {
        queue!(stdout, Print("\n"))?;
    }
    *input_row = (*input_row).saturating_sub(overflow);
    Ok(())
}

pub(super) fn move_after_repl_input(
    stdout: &mut io::Stdout,
    input_row: u16,
    rendered_rows: u16,
) -> Result<()> {
    queue!(
        stdout,
        MoveTo(0, input_row.saturating_add(rendered_rows.max(1)))
    )?;
    stdout.flush()?;
    Ok(())
}

/// 清除 REPL 可编辑输入区。
///
/// 参数:
/// - `stdout`: 终端输出
/// - `input_row`: 输入区起始行
/// - `rendered_rows`: 已渲染行数
///
/// 返回:
/// - 清除是否成功
pub(super) fn clear_repl_input(
    stdout: &mut io::Stdout,
    input_row: u16,
    rendered_rows: u16,
) -> Result<()> {
    for row_offset in 0..rendered_rows.max(1) {
        queue!(
            stdout,
            MoveTo(0, input_row.saturating_add(row_offset)),
            Clear(ClearType::CurrentLine)
        )?;
    }
    queue!(stdout, MoveTo(0, input_row))?;
    stdout.flush()?;
    Ok(())
}

#[allow(dead_code)]
pub(super) fn repl_render_rows(prefix: &str, lines: &[String], has_suggestions: bool) -> u16 {
    chrome_fixed_rows()
        + repl_prompt_rows_for_cols(prefix, lines, terminal_cols())
        + u16::from(has_suggestions)
}

pub(super) fn repl_prompt_rows(prefix: &str, lines: &[String]) -> u16 {
    repl_prompt_rows_for_cols(prefix, lines, terminal_cols())
}

pub(super) fn repl_cursor_position(prefix: &str, input: &str, cursor: usize) -> (u16, u16) {
    repl_cursor_position_for_cols(prefix, input, cursor, terminal_cols())
}

pub(super) fn repl_line_rows(prefix: &str, line: &str) -> u16 {
    repl_line_rows_for_cols(prefix, line, terminal_cols())
}

pub(super) fn repl_line_rows_for_cols(prefix: &str, line: &str, cols: usize) -> u16 {
    let cols = cols.max(1);
    let width = visible_width(prefix) + visible_width(line);
    (width / cols + 1).min(u16::MAX as usize) as u16
}

pub(super) fn repl_prompt_rows_for_cols(prefix: &str, lines: &[String], cols: usize) -> u16 {
    let cols = cols.max(1);
    if lines.is_empty() {
        return 1;
    }
    let mut rows = 0usize;
    for (index, line) in lines.iter().enumerate() {
        rows += repl_line_rows_for_cols(if index == 0 { prefix } else { "" }, line, cols) as usize;
    }
    rows.max(1).min(u16::MAX as usize) as u16
}

pub(super) fn repl_cursor_position_for_cols(
    prefix: &str,
    input: &str,
    cursor: usize,
    cols: usize,
) -> (u16, u16) {
    let cols = cols.max(1);
    let before_cursor = take_chars(input, cursor);
    let lines = repl_input_lines(&before_cursor);
    if lines.is_empty() {
        return (visible_width(prefix).min(u16::MAX as usize) as u16, 0);
    }
    let last_index = lines.len().saturating_sub(1);
    let mut row_offset = 0usize;
    for (index, line) in lines.iter().enumerate() {
        let width = if index == 0 {
            visible_width(prefix) + visible_width(line)
        } else {
            visible_width(line)
        };
        if index == last_index {
            return (
                (width % cols).min(u16::MAX as usize) as u16,
                (row_offset + width / cols).min(u16::MAX as usize) as u16,
            );
        }
        row_offset += width / cols + 1;
    }
    (visible_width(prefix).min(u16::MAX as usize) as u16, 0)
}
