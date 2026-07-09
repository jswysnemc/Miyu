use super::viewport::InlineViewport;
use crate::render::transcript::AnsiLine;
use anyhow::Result;
use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use std::io::{self, Write};

/// 将完整 transcript 重绘到 composer viewport 上方。
///
/// 参数:
/// - `stdout`: 终端输出句柄
/// - `viewport`: 当前 inline viewport
/// - `lines`: 已按当前宽度预换行的历史行
///
/// 返回:
/// - 写入是否成功
pub(super) fn replay_lines(
    stdout: &mut io::Stdout,
    viewport: &InlineViewport,
    lines: &[AnsiLine],
) -> Result<()> {
    replay_lines_to(stdout, viewport, lines)?;
    stdout.flush()?;
    Ok(())
}

/// 将新增稳定行插入历史区域底部。
///
/// 参数:
/// - `stdout`: 终端输出句柄
/// - `previous_viewport`: 追加前的 inline viewport
/// - `viewport`: 当前 inline viewport
/// - `lines`: 已按当前宽度预换行的新增历史行
///
/// 返回:
/// - 写入是否成功
pub(super) fn append_lines(
    stdout: &mut io::Stdout,
    previous_viewport: &InlineViewport,
    viewport: &InlineViewport,
    lines: &[AnsiLine],
) -> Result<()> {
    append_lines_to(stdout, previous_viewport, viewport, lines)?;
    stdout.flush()?;
    Ok(())
}

/// 将完整 transcript 按行定位到当前历史区域。
///
/// 参数:
/// - `output`: 支持终端控制序列的目标输出
/// - `viewport`: 当前 inline viewport
/// - `lines`: 已按当前宽度预换行的历史行
///
/// 返回:
/// - 写入是否成功
fn replay_lines_to<W: Write>(
    output: &mut W,
    viewport: &InlineViewport,
    lines: &[AnsiLine],
) -> Result<()> {
    let history_height = viewport.history_height();
    if history_height == 0 || lines.is_empty() {
        return Ok(());
    }

    let visible_start = lines.len().saturating_sub(usize::from(history_height));
    for (row, line) in lines[visible_start..].iter().enumerate() {
        queue!(
            output,
            MoveTo(0, viewport.origin_row().saturating_add(row as u16)),
            Print(line.as_str()),
            Clear(ClearType::UntilNewLine)
        )?;
    }
    Ok(())
}

/// 在 composer 上方追加稳定历史行。
///
/// 历史尚未填满终端时，直接写入旧 composer 顶部；填满后使用终端完整滚动区，
/// 使旧行进入原生 scrollback，而不是在受限 DECSTBM 区域内丢弃。
///
/// 参数:
/// - `output`: 支持终端控制序列的目标输出
/// - `previous_viewport`: 追加前的 inline viewport
/// - `viewport`: 当前 inline viewport
/// - `lines`: 已按当前宽度预换行的新增历史行
///
/// 返回:
/// - 写入是否成功
fn append_lines_to<W: Write>(
    output: &mut W,
    previous_viewport: &InlineViewport,
    viewport: &InlineViewport,
    lines: &[AnsiLine],
) -> Result<()> {
    if lines.is_empty() {
        return Ok(());
    }

    let previous_top = previous_viewport.composer_top();
    let composer_top = viewport.composer_top();
    let direct_rows = usize::from(composer_top.saturating_sub(previous_top));
    let direct_line_count = lines.len().min(direct_rows);
    for (index, line) in lines[..direct_line_count].iter().enumerate() {
        queue!(
            output,
            MoveTo(0, previous_top.saturating_add(index as u16)),
            Print(line.as_str()),
            Clear(ClearType::UntilNewLine)
        )?;
    }
    if direct_line_count == lines.len() {
        return Ok(());
    }

    for line in &lines[direct_line_count..] {
        queue!(
            output,
            // 1. 在屏幕最后一行换行，交由终端把旧屏幕行写入原生 scrollback
            MoveTo(0, previous_viewport.size().rows.saturating_sub(1)),
            Print("\r\n"),
            // 2. 末条历史固定在 composer 紧上方，随后由 composer 重绘覆盖旧输入区
            MoveTo(0, composer_top.saturating_sub(1)),
            Print(line.as_str()),
            Clear(ClearType::UntilNewLine)
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{append_lines_to, replay_lines_to};
    use crate::cli::repl_runtime::viewport::{InlineViewport, TerminalSize};
    use crate::render::transcript::AnsiLine;

    /// 验证完整重放逐行定位，不追加导致首行滚出的换行。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn replay_lines_do_not_scroll_first_history_row() {
        let mut viewport = InlineViewport::new();
        viewport.update(TerminalSize { cols: 80, rows: 24 }, 3, 4);
        let mut output = Vec::new();
        let lines = vec![
            AnsiLine::new("first".to_string()),
            AnsiLine::new("second".to_string()),
        ];

        replay_lines_to(&mut output, &viewport, &lines).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\x1b[1;1Hfirst\x1b[K\x1b[2;1Hsecond\x1b[K"));
        assert!(!output.contains("\r\n"));
    }

    /// 验证历史未填满时，新行直接写入旧 composer 顶部。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn appended_lines_follow_short_history() {
        let mut previous = InlineViewport::new();
        previous.update(TerminalSize { cols: 80, rows: 24 }, 3, 4);
        let mut viewport = InlineViewport::new();
        viewport.update(TerminalSize { cols: 80, rows: 24 }, 3, 5);
        let mut output = Vec::new();

        append_lines_to(
            &mut output,
            &previous,
            &viewport,
            &[AnsiLine::new("next".to_string())],
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\x1b[5;1Hnext\x1b[K"));
        assert!(!output.contains("\x1b[r"));
    }

    /// 验证历史填满后使用完整终端滚动，保留原生 scrollback。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn appended_lines_scroll_the_full_terminal_after_history_fills() {
        let mut previous = InlineViewport::new();
        previous.update(TerminalSize { cols: 80, rows: 24 }, 3, 80);
        let mut viewport = InlineViewport::new();
        viewport.update(TerminalSize { cols: 80, rows: 24 }, 3, 81);
        let mut output = Vec::new();

        append_lines_to(
            &mut output,
            &previous,
            &viewport,
            &[AnsiLine::new("next".to_string())],
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\x1b[24;1H\r\n\x1b[21;1Hnext\x1b[K"));
    }

    /// 验证大块追加先填满空余历史行，再进入终端 scrollback。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    #[test]
    fn large_append_uses_scrollback_after_filling_open_history_rows() {
        let mut previous = InlineViewport::new();
        previous.update(TerminalSize { cols: 80, rows: 24 }, 3, 4);
        let mut viewport = InlineViewport::new();
        viewport.update(TerminalSize { cols: 80, rows: 24 }, 3, 22);
        let lines = (0..18)
            .map(|index| AnsiLine::new(format!("line-{index}")))
            .collect::<Vec<_>>();
        let mut output = Vec::new();

        append_lines_to(&mut output, &previous, &viewport, &lines).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\x1b[5;1Hline-0\x1b[K"));
        assert!(output.contains("\x1b[21;1Hline-16\x1b[K"));
        assert!(output.contains("\x1b[24;1H\r\n\x1b[21;1Hline-17\x1b[K"));
    }
}
