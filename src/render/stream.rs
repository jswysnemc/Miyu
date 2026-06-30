use crate::i18n::text as t;
use crate::llm::{ChatStreamChunk, ChatStreamKind};
use crate::render::command_output::{
    write_command_block, write_command_error_block, write_command_result_blocks, write_tool_payload,
};
use crate::render::markdown::MarkdownStreamRenderer;
use crate::render::stream_summary::StreamSummary;
use crate::render::wait_spinner::WaitSpinner;
use anyhow::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use std::io::{self, Write};

#[cfg(test)]
use crate::render::stream_summary::{
    style_summary_text, tool_status_text, SummaryStyle, ToolStats,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasoningDisplayMode {
    Hidden,
    Summary,
    Full,
}

impl ReasoningDisplayMode {
    /// 从配置文本解析推理展示模式。
    ///
    /// 参数:
    /// - `value`: 配置值
    ///
    /// 返回:
    /// - 推理展示模式
    pub fn from_config(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "hidden" => Self::Hidden,
            "full" => Self::Full,
            _ => Self::Summary,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolCallDisplayMode {
    Hidden,
    Summary,
    Full,
}

impl ToolCallDisplayMode {
    /// 从配置文本解析工具调用展示模式。
    ///
    /// 参数:
    /// - `value`: 配置值
    ///
    /// 返回:
    /// - 工具调用展示模式
    pub fn from_config(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "hidden" => Self::Hidden,
            "full" => Self::Full,
            _ => Self::Summary,
        }
    }
}

pub struct StreamRenderer {
    reasoning_mode: ReasoningDisplayMode,
    tool_call_mode: ToolCallDisplayMode,
    plain: bool,
    mode: Option<ChatStreamKind>,
    cursor_hidden: bool,
    markdown: MarkdownStreamRenderer,
    summary: StreamSummary,
    wait_spinner: Option<WaitSpinner>,
}

impl StreamRenderer {
    /// 创建流式响应渲染器。
    ///
    /// 参数:
    /// - `reasoning_mode`: 推理内容展示模式
    /// - `tool_call_mode`: 工具调用展示模式
    /// - `plain`: 是否使用纯文本输出
    /// - `readable_tool_names`: 是否展示可读工具名称
    ///
    /// 返回:
    /// - 新的流式渲染器
    pub fn new(
        reasoning_mode: ReasoningDisplayMode,
        tool_call_mode: ToolCallDisplayMode,
        plain: bool,
        readable_tool_names: bool,
    ) -> Self {
        Self {
            reasoning_mode,
            tool_call_mode,
            plain,
            mode: None,
            cursor_hidden: false,
            markdown: MarkdownStreamRenderer::new(),
            summary: StreamSummary::new(readable_tool_names),
            wait_spinner: None,
        }
    }

    /// 启动等待响应动画。
    ///
    /// 返回:
    /// - 启动是否成功
    pub fn start_waiting(&mut self) -> Result<()> {
        if self.plain || self.wait_spinner.is_some() || !WaitSpinner::supported() {
            return Ok(());
        }
        self.hide_cursor()?;
        self.wait_spinner = Some(WaitSpinner::start(t("thinking", "思考").to_string()));
        Ok(())
    }

    /// 写入模型流式文本片段。
    ///
    /// 参数:
    /// - `chunk`: 模型流式片段
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn write_chunk(&mut self, chunk: ChatStreamChunk) -> Result<()> {
        self.hide_cursor()?;
        let text = normalize_stream_text(&chunk.text);
        if self.plain && chunk.kind == ChatStreamKind::Reasoning {
            return Ok(());
        }
        if self.reasoning_mode == ReasoningDisplayMode::Hidden
            && chunk.kind == ChatStreamKind::Reasoning
        {
            return Ok(());
        }
        if self.reasoning_mode == ReasoningDisplayMode::Summary
            && chunk.kind == ChatStreamKind::Reasoning
        {
            self.finalize_tools_summary()?;
            self.summary.add_reasoning_text(&text);
            self.mode = Some(ChatStreamKind::Reasoning);
            if self.wait_spinner.is_some() {
                self.summary.clear_live_lines()?;
                self.set_waiting_phase(self.summary.reasoning_text());
            } else {
                self.summary.render_reasoning_live()?;
            }
            return Ok(());
        }
        self.stop_waiting()?;
        if self.mode != Some(chunk.kind) {
            if chunk.kind == ChatStreamKind::Content {
                self.finalize_reasoning_summary()?;
                self.finalize_tools_summary()?;
            }
            self.switch_mode(chunk.kind)?;
        }
        let mut stdout = io::stdout();
        if self.plain || chunk.kind == ChatStreamKind::Reasoning {
            write!(stdout, "{text}")?;
        } else {
            write!(stdout, "{}", self.markdown.push(&text))?;
        }
        stdout.flush()?;
        Ok(())
    }

    /// 写入工具调用。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `arguments`: 工具参数
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn write_tool_call(&mut self, name: &str, arguments: &str) -> Result<()> {
        if self.plain {
            return Ok(());
        }
        self.stop_waiting()?;
        self.end_active_stream_line()?;
        self.finalize_reasoning_summary()?;
        if name == "run_command" {
            let mut stdout = io::stdout();
            write_command_block(&mut stdout, arguments)?;
            stdout.flush()?;
            if self.tool_call_mode == ToolCallDisplayMode::Summary {
                self.summary.note_tool_call(name);
            }
            return Ok(());
        }
        if self.tool_call_mode == ToolCallDisplayMode::Full {
            let mut stdout = io::stdout();
            writeln!(stdout, "tool {}", self.summary.display_tool_name(name))?;
            write_tool_payload(&mut stdout, t("args", "参数"), arguments)?;
            stdout.flush()?;
        } else if self.tool_call_mode == ToolCallDisplayMode::Summary {
            self.summary.note_tool_call(name);
            self.summary.render_tools_live()?;
        }
        Ok(())
    }

    /// 写入工具结果。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `ok`: 工具是否成功
    /// - `output`: 工具输出
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn write_tool_result(&mut self, name: &str, ok: bool, output: &str) -> Result<()> {
        if self.plain {
            return Ok(());
        }
        self.stop_waiting()?;
        let status = if ok { "ok" } else { "err" };
        if name == "run_command" {
            if self.tool_call_mode == ToolCallDisplayMode::Summary {
                self.summary.note_tool_result(name, ok);
                if !ok {
                    let mut stdout = io::stdout();
                    write_command_error_block(&mut stdout, output)?;
                    stdout.flush()?;
                }
                return Ok(());
            }
            if self.tool_call_mode == ToolCallDisplayMode::Full {
                let mut stdout = io::stdout();
                write_command_result_blocks(&mut stdout, output)?;
                stdout.flush()?;
                return Ok(());
            }
        }
        if self.tool_call_mode == ToolCallDisplayMode::Full {
            let mut stdout = io::stdout();
            writeln!(
                stdout,
                "result {} {status}",
                self.summary.display_tool_name(name)
            )?;
            write_tool_payload(&mut stdout, t("output", "输出"), output)?;
            stdout.flush()?;
        } else if self.tool_call_mode == ToolCallDisplayMode::Summary {
            self.summary.note_tool_result(name, ok);
            self.summary.render_tools_live()?;
        }
        Ok(())
    }

    /// 写入工具进度。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `message`: 进度信息
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn write_tool_progress(&mut self, name: &str, message: &str) -> Result<()> {
        if self.plain {
            return Ok(());
        }
        if message == "__external_output__" {
            self.prepare_for_external_output()?;
            return Ok(());
        }
        self.stop_waiting()?;
        self.end_active_stream_line()?;
        self.finalize_reasoning_summary()?;
        if self.tool_call_mode == ToolCallDisplayMode::Full {
            let mut stdout = io::stdout();
            writeln!(
                stdout,
                "progress {}: {message}",
                self.summary.display_tool_name(name)
            )?;
            stdout.flush()?;
        } else if self.tool_call_mode == ToolCallDisplayMode::Summary {
            self.summary.note_tool_progress(name, message);
            self.summary.render_tools_live()?;
        }
        Ok(())
    }

    /// 为外部程序直接输出终端内容做准备。
    ///
    /// 返回:
    /// - 准备是否成功
    pub fn prepare_for_external_output(&mut self) -> Result<()> {
        self.stop_waiting()?;
        self.summary.clear_live_lines()?;
        self.end_active_stream_line()?;
        self.finalize_reasoning_summary()?;
        self.finalize_tools_summary()?;
        self.show_cursor()?;
        Ok(())
    }

    /// 完成当前流式渲染。
    ///
    /// 返回:
    /// - 收尾是否成功
    pub fn finish(&mut self) -> Result<()> {
        self.stop_waiting()?;
        self.summary.clear_live_lines()?;
        if self.mode == Some(ChatStreamKind::Content) && !self.plain {
            let mut stdout = io::stdout();
            write!(stdout, "{}", self.markdown.flush())?;
            stdout.flush()?;
        }
        if self.mode == Some(ChatStreamKind::Reasoning) {
            execute!(io::stdout(), ResetColor)?;
        }
        if self.mode.is_some() {
            println!();
        }
        self.finalize_reasoning_summary()?;
        self.finalize_tools_summary()?;
        self.mode = None;
        self.show_cursor()?;
        Ok(())
    }

    /// 切换当前流式输出模式。
    ///
    /// 参数:
    /// - `mode`: 新输出模式
    ///
    /// 返回:
    /// - 切换是否成功
    fn switch_mode(&mut self, mode: ChatStreamKind) -> Result<()> {
        let mut stdout = io::stdout();
        match mode {
            ChatStreamKind::Reasoning => {
                if self.mode.is_some() {
                    writeln!(stdout)?;
                }
                execute!(stdout, SetForegroundColor(Color::DarkCyan))?;
                writeln!(stdout, "{}", t("thinking", "思考"))?;
            }
            ChatStreamKind::Content => {
                if self.mode == Some(ChatStreamKind::Reasoning) {
                    execute!(stdout, ResetColor)?;
                    writeln!(stdout)?;
                }
            }
        }
        stdout.flush()?;
        self.mode = Some(mode);
        Ok(())
    }

    /// 结束当前活动流式行。
    ///
    /// 返回:
    /// - 结束是否成功
    fn end_active_stream_line(&mut self) -> Result<()> {
        if self.reasoning_mode == ReasoningDisplayMode::Summary
            && self.mode == Some(ChatStreamKind::Reasoning)
        {
            self.mode = None;
            return Ok(());
        }
        if self.mode == Some(ChatStreamKind::Reasoning) {
            execute!(io::stdout(), ResetColor)?;
        } else if self.mode == Some(ChatStreamKind::Content) && !self.plain {
            let mut stdout = io::stdout();
            write!(stdout, "{}", self.markdown.flush())?;
            stdout.flush()?;
        }
        if self.mode.is_some() {
            println!();
            self.mode = None;
        }
        Ok(())
    }

    /// 固化推理摘要。
    ///
    /// 返回:
    /// - 固化是否成功
    fn finalize_reasoning_summary(&mut self) -> Result<()> {
        if self.reasoning_mode == ReasoningDisplayMode::Summary && self.summary.has_reasoning() {
            self.stop_waiting()?;
            self.summary.finalize_reasoning()?;
            self.mode = None;
        }
        Ok(())
    }

    /// 固化工具调用摘要。
    ///
    /// 返回:
    /// - 固化是否成功
    fn finalize_tools_summary(&mut self) -> Result<()> {
        if self.tool_call_mode == ToolCallDisplayMode::Summary && self.summary.has_tools() {
            self.stop_waiting()?;
            self.summary.finalize_tools()?;
        }
        Ok(())
    }

    /// 更新等待动画状态文本。
    ///
    /// 参数:
    /// - `phase`: 新状态文本
    fn set_waiting_phase(&mut self, phase: String) {
        if let Some(spinner) = &self.wait_spinner {
            spinner.set_phase(phase);
        }
    }

    /// 停止等待动画。
    ///
    /// 返回:
    /// - 停止是否成功
    fn stop_waiting(&mut self) -> Result<()> {
        if let Some(mut spinner) = self.wait_spinner.take() {
            spinner.stop()?;
        }
        Ok(())
    }

    /// 隐藏终端光标。
    ///
    /// 返回:
    /// - 操作是否成功
    fn hide_cursor(&mut self) -> Result<()> {
        if !self.cursor_hidden {
            execute!(io::stdout(), Hide)?;
            self.cursor_hidden = true;
        }
        Ok(())
    }

    /// 显示终端光标。
    ///
    /// 返回:
    /// - 操作是否成功
    fn show_cursor(&mut self) -> Result<()> {
        if self.cursor_hidden {
            execute!(io::stdout(), Show)?;
            self.cursor_hidden = false;
        }
        Ok(())
    }
}

impl Drop for StreamRenderer {
    fn drop(&mut self) {
        let _ = self.stop_waiting();
        let _ = self.summary.clear_live_lines();
        let _ = self.show_cursor();
        let _ = execute!(io::stdout(), ResetColor);
    }
}

/// 归一化流式文本换行。
///
/// 参数:
/// - `text`: 原始文本
///
/// 返回:
/// - 使用 `\n` 的文本
fn normalize_stream_text(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
