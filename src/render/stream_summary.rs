use crate::i18n::text as t;
use crate::render::tool_names::readable_tool_name;
use anyhow::Result;
use crossterm::cursor::{MoveToColumn, MoveUp};
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use std::collections::BTreeMap;
use std::io::{self, IsTerminal, Write};

pub(crate) struct StreamSummary {
    reasoning_chars: usize,
    reasoning_lines: usize,
    tool_stats: BTreeMap<String, ToolStats>,
    readable_tool_names: bool,
    live_summary: bool,
    line_active: bool,
    lines_active: u16,
}

impl StreamSummary {
    /// 创建流式摘要状态。
    ///
    /// 参数:
    /// - `readable_tool_names`: 是否展示可读工具名称
    ///
    /// 返回:
    /// - 新的流式摘要状态
    pub(crate) fn new(readable_tool_names: bool) -> Self {
        Self {
            reasoning_chars: 0,
            reasoning_lines: 0,
            tool_stats: BTreeMap::new(),
            readable_tool_names,
            live_summary: io::stdout().is_terminal(),
            line_active: false,
            lines_active: 0,
        }
    }

    /// 累加推理摘要计数。
    ///
    /// 参数:
    /// - `text`: 本次收到的推理文本
    pub(crate) fn add_reasoning_text(&mut self, text: &str) {
        self.reasoning_chars += text.chars().count();
        self.reasoning_lines += text.matches('\n').count();
    }

    /// 判断是否存在待固化的推理摘要。
    ///
    /// 返回:
    /// - 是否存在推理摘要
    pub(crate) fn has_reasoning(&self) -> bool {
        self.reasoning_chars > 0
    }

    /// 判断是否存在待固化的工具摘要。
    ///
    /// 返回:
    /// - 是否存在工具摘要
    pub(crate) fn has_tools(&self) -> bool {
        !self.tool_stats.is_empty()
    }

    /// 生成推理摘要文本。
    ///
    /// 返回:
    /// - 推理摘要文本
    pub(crate) fn reasoning_text(&self) -> String {
        format!(
            "{} · {} {} · {} {}",
            t("thinking", "思考"),
            self.reasoning_lines.max(1),
            t("lines", "行"),
            self.reasoning_chars,
            t("chars", "字符")
        )
    }

    /// 渲染实时推理摘要行。
    ///
    /// 返回:
    /// - 渲染是否成功
    pub(crate) fn render_reasoning_live(&mut self) -> Result<()> {
        let text = self.reasoning_text();
        self.render_live_line(&text, SummaryStyle::Reasoning)
    }

    /// 固化推理摘要。
    ///
    /// 返回:
    /// - 固化是否成功
    pub(crate) fn finalize_reasoning(&mut self) -> Result<()> {
        if !self.has_reasoning() {
            return Ok(());
        }
        let text = self.reasoning_text();
        if self.line_active {
            let mut stdout = io::stdout();
            self.clear_live_lines()?;
            writeln!(
                stdout,
                "{}",
                style_summary_text(&text, SummaryStyle::Reasoning)
            )?;
            stdout.flush()?;
        } else {
            println!("{}", style_summary_text(&text, SummaryStyle::Reasoning));
        }
        self.reasoning_chars = 0;
        self.reasoning_lines = 0;
        Ok(())
    }

    /// 记录工具调用开始。
    ///
    /// 参数:
    /// - `name`: 工具名称
    pub(crate) fn note_tool_call(&mut self, name: &str) {
        self.tool_stats.entry(name.to_string()).or_default().calls += 1;
    }

    /// 记录工具调用结果。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `ok`: 工具是否成功
    pub(crate) fn note_tool_result(&mut self, name: &str, ok: bool) {
        let stats = self.tool_stats.entry(name.to_string()).or_default();
        if ok {
            stats.ok += 1;
        } else {
            stats.error += 1;
            stats.progress = None;
        }
    }

    /// 更新工具调用进度。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `message`: 进度信息
    pub(crate) fn note_tool_progress(&mut self, name: &str, message: &str) {
        self.tool_stats
            .entry(name.to_string())
            .or_default()
            .progress = Some(message.to_string());
    }

    /// 渲染实时工具摘要行。
    ///
    /// 返回:
    /// - 渲染是否成功
    pub(crate) fn render_tools_live(&mut self) -> Result<()> {
        let text = self.tool_summary_text();
        self.render_live_line(&text, SummaryStyle::Tool)
    }

    /// 固化工具调用摘要。
    ///
    /// 返回:
    /// - 固化是否成功
    pub(crate) fn finalize_tools(&mut self) -> Result<()> {
        if self.tool_stats.is_empty() {
            return Ok(());
        }
        let text = self.tool_summary_text();
        if self.line_active {
            let mut stdout = io::stdout();
            self.clear_live_lines()?;
            writeln!(stdout, "{}", style_summary_text(&text, SummaryStyle::Tool))?;
            stdout.flush()?;
        } else {
            println!("{}", style_summary_text(&text, SummaryStyle::Tool));
        }
        self.tool_stats.clear();
        Ok(())
    }

    /// 清除当前实时摘要行。
    ///
    /// 返回:
    /// - 清除是否成功
    pub(crate) fn clear_live_lines(&mut self) -> Result<()> {
        if !self.line_active {
            return Ok(());
        }
        let mut stdout = io::stdout();
        let lines = self.lines_active.max(1);
        for index in 0..lines {
            if index > 0 {
                execute!(stdout, MoveUp(1))?;
            }
            execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
        }
        stdout.flush()?;
        self.line_active = false;
        self.lines_active = 0;
        Ok(())
    }

    /// 返回展示用工具名称。
    ///
    /// 参数:
    /// - `name`: 工具原始名称
    ///
    /// 返回:
    /// - 展示名称
    pub(crate) fn display_tool_name<'a>(&self, name: &'a str) -> &'a str {
        if self.readable_tool_names {
            readable_tool_name(name)
        } else {
            name
        }
    }

    /// 渲染实时摘要行。
    ///
    /// 参数:
    /// - `text`: 摘要文本
    /// - `style`: 摘要样式
    ///
    /// 返回:
    /// - 渲染是否成功
    fn render_live_line(&mut self, text: &str, style: SummaryStyle) -> Result<()> {
        if !self.live_summary {
            return Ok(());
        }
        let mut stdout = io::stdout();
        self.clear_live_lines()?;
        let lines = text.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                writeln!(stdout)?;
            }
            write!(stdout, "{}\x1b[K", style_summary_text(line, style))?;
        }
        stdout.flush()?;
        self.line_active = true;
        self.lines_active = lines.len().max(1) as u16;
        Ok(())
    }

    /// 生成工具调用摘要文本。
    ///
    /// 返回:
    /// - 工具调用摘要文本
    fn tool_summary_text(&self) -> String {
        let parts = self
            .tool_stats
            .iter()
            .map(|(name, stats)| {
                let header = tool_status_text(self.display_tool_name(name), stats);
                stats.progress.as_ref().map_or(header.clone(), |message| {
                    let progress = message
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .map(|line| format!("· {}", clip_progress_line(line, 120)))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if progress.is_empty() {
                        header
                    } else {
                        format!("{header}\n{progress}")
                    }
                })
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}: {parts}", t("tools", "工具"))
    }
}

#[derive(Default)]
pub(crate) struct ToolStats {
    pub(crate) calls: usize,
    pub(crate) ok: usize,
    pub(crate) error: usize,
    pub(crate) progress: Option<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum SummaryStyle {
    Reasoning,
    Tool,
}

/// 为摘要文本添加终端样式。
///
/// 参数:
/// - `text`: 摘要文本
/// - `style`: 摘要类型
///
/// 返回:
/// - 带 ANSI 样式的摘要文本
pub(crate) fn style_summary_text(text: &str, style: SummaryStyle) -> String {
    match style {
        SummaryStyle::Reasoning => format!("\x1b[2m\x1b[36m{text}\x1b[0m"),
        SummaryStyle::Tool => format!("\x1b[2m{text}\x1b[0m"),
    }
}

/// 生成工具状态文本。
///
/// 参数:
/// - `name`: 工具展示名称
/// - `stats`: 工具调用统计
///
/// 返回:
/// - 工具状态摘要
pub(crate) fn tool_status_text(name: &str, stats: &ToolStats) -> String {
    let calls = stats.calls.max(stats.ok + stats.error).max(1);
    let running = stats.calls.saturating_sub(stats.ok + stats.error);
    if calls == 1 {
        if running > 0 {
            return format!("{name}×1 {}", t("running", "运行中"));
        }
        if stats.error > 0 {
            return format!("{name}×1 err");
        }
        if stats.ok > 0 {
            return format!("{name}×1 ok");
        }
    }
    if running > 0 {
        format!(
            "{name}×{calls} {}:{} ok:{} err:{}",
            t("running", "运行中"),
            running,
            stats.ok,
            stats.error
        )
    } else {
        format!("{name}×{calls} ok:{} err:{}", stats.ok, stats.error)
    }
}

/// 压缩进度文本为单行。
///
/// 参数:
/// - `text`: 原始文本
/// - `max_chars`: 最大字符数
///
/// 返回:
/// - 压缩后的文本
fn clip_progress_line(text: &str, max_chars: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max_chars {
        text
    } else {
        format!(
            "{}...",
            text.chars()
                .take(max_chars.saturating_sub(3))
                .collect::<String>()
        )
    }
}
