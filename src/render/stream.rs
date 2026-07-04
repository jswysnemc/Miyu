use crate::i18n::text as t;
use crate::llm::{ChatStreamChunk, ChatStreamKind, ToolCallStreamProgress};
use crate::render::command_output::{
    write_command_block, write_command_error_block, write_command_result_blocks,
    write_edit_file_diff_block, write_tool_payload, write_write_file_diff_block,
};
use crate::render::live_tool_status::LiveToolStatus;
use crate::render::markdown::MarkdownStreamRenderer;
use crate::render::stream_summary::StreamSummary;
pub(crate) use crate::render::stream_text::{
    normalize_stream_text, tool_call_has_visible_block, tool_event_text, wait_spinner_detail_line,
};
use crate::render::style::TOOL_BULLET;
use crate::render::wait_spinner::{SpinnerStyle, WaitSpinner};
use crate::tools::command::background_render::is_background_command_start;
use anyhow::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use serde_json::Value;
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
    options: StreamRenderOptions,
    mode: Option<ChatStreamKind>,
    cursor_hidden: bool,
    markdown: MarkdownStreamRenderer,
    summary: StreamSummary,
    wait_spinner: Option<WaitSpinner>,
    live_tool_status: LiveToolStatus,
}

#[derive(Clone, Debug, Default)]
pub struct StreamRenderOptions {
    pub readable_tool_names: bool,
    pub wait_model: Option<String>,
    pub wait_thinking_level: Option<String>,
}

impl StreamRenderer {
    /// 创建流式响应渲染器。
    ///
    /// 参数:
    /// - `reasoning_mode`: 推理内容展示模式
    /// - `tool_call_mode`: 工具调用展示模式
    /// - `plain`: 是否使用纯文本输出
    /// - `options`: 流式渲染附加选项
    ///
    /// 返回:
    /// - 新的流式渲染器
    pub fn new(
        reasoning_mode: ReasoningDisplayMode,
        tool_call_mode: ToolCallDisplayMode,
        plain: bool,
        options: StreamRenderOptions,
    ) -> Self {
        let readable_tool_names = options.readable_tool_names;
        Self {
            reasoning_mode,
            tool_call_mode,
            plain,
            options,
            mode: None,
            cursor_hidden: false,
            markdown: MarkdownStreamRenderer::new(),
            summary: StreamSummary::new(readable_tool_names),
            wait_spinner: None,
            live_tool_status: LiveToolStatus::new(),
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
        self.wait_spinner = Some(WaitSpinner::start(
            t("thinking", "思考").to_string(),
            SpinnerStyle::Scanner,
            wait_spinner_detail_line(&self.options),
        ));
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
            self.stop_waiting()?;
            self.finish_live_tool_status()?;
            self.finalize_tools_summary()?;
            self.summary.add_reasoning_text(&text);
            self.mode = Some(ChatStreamKind::Reasoning);
            return Ok(());
        }
        self.stop_waiting()?;
        self.finish_live_tool_status()?;
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
        let background_command_start =
            name == "background_command" && is_background_command_start(arguments);
        if self.tool_call_mode == ToolCallDisplayMode::Summary
            && !tool_call_has_visible_block(name)
            && !background_command_start
        {
            self.write_live_tool_status(name, "run", false)?;
            return Ok(());
        }
        self.clear_live_tool_status()?;
        self.end_active_stream_line()?;
        self.finalize_reasoning_summary()?;
        if name == "run_command" || background_command_start {
            self.summary.clear_live_lines()?;
            let mut stdout = io::stdout();
            write_command_block(&mut stdout, arguments)?;
            stdout.flush()?;
            return Ok(());
        }
        if name == "edit_file" {
            self.summary.clear_live_lines()?;
            let mut stdout = io::stdout();
            if !write_edit_file_diff_block(&mut stdout, arguments)? {
                write_tool_payload(&mut stdout, t("args", "参数"), arguments)?;
            }
            stdout.flush()?;
            return Ok(());
        }
        if name == "write_file" && self.tool_call_mode != ToolCallDisplayMode::Hidden {
            self.summary.clear_live_lines()?;
            let mut stdout = io::stdout();
            if !write_write_file_diff_block(&mut stdout, arguments)? {
                write_tool_payload(&mut stdout, t("args", "参数"), arguments)?;
            }
            stdout.flush()?;
            return Ok(());
        }
        if self.tool_call_mode == ToolCallDisplayMode::Full {
            self.summary.clear_live_lines()?;
            let mut stdout = io::stdout();
            writeln!(
                stdout,
                "{TOOL_BULLET} tool {}",
                self.summary.display_tool_name(name)
            )?;
            write_tool_payload(&mut stdout, t("args", "参数"), arguments)?;
            stdout.flush()?;
        }
        Ok(())
    }

    /// 写入工具调用参数接收进度。
    ///
    /// 参数:
    /// - `progress`: 工具调用参数流式进度
    ///
    /// 返回:
    /// - 写入是否成功
    pub fn write_tool_call_progress(&mut self, progress: &ToolCallStreamProgress) -> Result<()> {
        if self.plain {
            return Ok(());
        }
        self.stop_waiting()?;
        if self.tool_call_mode == ToolCallDisplayMode::Hidden {
            return Ok(());
        }
        let name = progress.name.as_deref().unwrap_or("tool");
        self.write_live_tool_status(name, "arg", false)
    }

    /// 写入单行工具状态。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `status`: 工具状态，取值为 arg、run、ok 或 err
    /// - `final_line`: 是否结束当前状态行
    ///
    /// 返回:
    /// - 写入是否成功
    fn write_live_tool_status(&mut self, name: &str, status: &str, final_line: bool) -> Result<()> {
        if !self.live_tool_status.is_active() {
            self.end_active_stream_line()?;
            self.finalize_reasoning_summary()?;
        }
        self.live_tool_status
            .write(self.summary.display_tool_name(name), status, final_line)
    }

    /// 结束当前单行工具状态。
    ///
    /// 返回:
    /// - 写入是否成功
    fn finish_live_tool_status(&mut self) -> Result<()> {
        self.live_tool_status.finish()
    }

    /// 清除当前单行工具状态。
    ///
    /// 返回:
    /// - 写入是否成功
    fn clear_live_tool_status(&mut self) -> Result<()> {
        self.live_tool_status.clear()
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
        if self.tool_call_mode == ToolCallDisplayMode::Summary {
            if tool_call_has_visible_block(name) {
                if name == "run_command" && !ok {
                    let mut stdout = io::stdout();
                    write_command_error_block(&mut stdout, output)?;
                    stdout.flush()?;
                }
                self.write_tool_event_line(name, status)?;
                return Ok(());
            }
            self.write_live_tool_status(name, status, true)?;
            return Ok(());
        }
        self.finish_live_tool_status()?;
        if name == "run_command" {
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
                "{TOOL_BULLET} result {} {status}",
                self.summary.display_tool_name(name)
            )?;
            write_tool_payload(&mut stdout, t("output", "输出"), output)?;
            stdout.flush()?;
        } else if self.tool_call_mode == ToolCallDisplayMode::Summary {
            self.summary.note_tool_result(name, ok);
            if !tool_call_has_visible_block(name) {
                self.write_tool_event_line(name, status)?;
            }
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
        if let Some(text) = message.strip_prefix("__subagent_reasoning__") {
            return self.write_subagent_reasoning(name, text);
        }
        if let Some(payload) = message.strip_prefix("__subtool_call__") {
            return self.write_subtool_call(name, payload);
        }
        if let Some(payload) = message.strip_prefix("__subtool_result__") {
            return self.write_subtool_result(name, payload);
        }
        self.stop_waiting()?;
        self.end_active_stream_line()?;
        self.finalize_reasoning_summary()?;
        if self.tool_call_mode == ToolCallDisplayMode::Full {
            let mut stdout = io::stdout();
            writeln!(
                stdout,
                "{TOOL_BULLET} progress {}: {message}",
                self.summary.display_tool_name(name)
            )?;
            stdout.flush()?;
        } else if self.tool_call_mode == ToolCallDisplayMode::Summary {
            self.summary.note_tool_progress(name, message);
        }
        Ok(())
    }

    /// 写入子代理推理进度。
    ///
    /// 参数:
    /// - `name`: 父工具名称
    /// - `text`: 子代理推理文本
    ///
    /// 返回:
    /// - 写入是否成功
    fn write_subagent_reasoning(&mut self, name: &str, text: &str) -> Result<()> {
        let text = normalize_stream_text(text);
        if self.tool_call_mode == ToolCallDisplayMode::Hidden || text.trim().is_empty() {
            return Ok(());
        }
        if self.tool_call_mode == ToolCallDisplayMode::Full {
            self.stop_waiting()?;
            self.end_active_stream_line()?;
            self.finalize_reasoning_summary()?;
            let mut stdout = io::stdout();
            execute!(stdout, SetForegroundColor(Color::Green))?;
            write!(stdout, "{text}")?;
            execute!(stdout, ResetColor)?;
            stdout.flush()?;
            return Ok(());
        }
        let line_count = text.matches('\n').count().max(1);
        self.summary.note_tool_progress(
            name,
            &format!(
                "{} · {} {}",
                t("subagent reasoning", "子代理推理"),
                line_count,
                t("lines", "行")
            ),
        );
        Ok(())
    }

    /// 写入子工具调用进度。
    ///
    /// 参数:
    /// - `parent_name`: 父工具名称
    /// - `payload`: 子工具调用 JSON
    ///
    /// 返回:
    /// - 写入是否成功
    fn write_subtool_call(&mut self, parent_name: &str, payload: &str) -> Result<()> {
        let value = serde_json::from_str::<Value>(payload).unwrap_or(Value::Null);
        let tool_name = value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let args = value.get("args").and_then(Value::as_str).unwrap_or("");
        if self.tool_call_mode == ToolCallDisplayMode::Full {
            self.stop_waiting()?;
            self.end_active_stream_line()?;
            self.finalize_reasoning_summary()?;
            let mut stdout = io::stdout();
            if tool_name == "run_command" {
                write_command_block(&mut stdout, args)?;
            } else if tool_name == "edit_file" {
                if !write_edit_file_diff_block(&mut stdout, args)? {
                    write_tool_payload(&mut stdout, t("args", "参数"), args)?;
                }
            } else {
                writeln!(
                    stdout,
                    "{TOOL_BULLET} tool {}",
                    self.summary.display_tool_name(tool_name)
                )?;
                write_tool_payload(&mut stdout, t("args", "参数"), args)?;
            }
            stdout.flush()?;
        } else if self.tool_call_mode == ToolCallDisplayMode::Summary {
            self.summary.note_tool_progress(
                parent_name,
                &format!(
                    "{}: {}",
                    t("subtool running", "子工具运行中"),
                    self.summary.display_tool_name(tool_name)
                ),
            );
        }
        Ok(())
    }

    /// 写入子工具结果进度。
    ///
    /// 参数:
    /// - `parent_name`: 父工具名称
    /// - `payload`: 子工具结果 JSON
    ///
    /// 返回:
    /// - 写入是否成功
    fn write_subtool_result(&mut self, parent_name: &str, payload: &str) -> Result<()> {
        let value = serde_json::from_str::<Value>(payload).unwrap_or(Value::Null);
        let tool_name = value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let ok = value.get("ok").and_then(Value::as_bool).unwrap_or(true);
        let output = value.get("output").and_then(Value::as_str).unwrap_or("");
        let status = if ok { "ok" } else { "err" };
        if self.tool_call_mode == ToolCallDisplayMode::Full {
            self.stop_waiting()?;
            self.end_active_stream_line()?;
            self.finalize_reasoning_summary()?;
            let mut stdout = io::stdout();
            if tool_name == "run_command" {
                write_command_result_blocks(&mut stdout, output)?;
            } else {
                writeln!(
                    stdout,
                    "{TOOL_BULLET} result {} {status}",
                    self.summary.display_tool_name(tool_name)
                )?;
                write_tool_payload(&mut stdout, t("output", "输出"), output)?;
            }
            stdout.flush()?;
        } else if self.tool_call_mode == ToolCallDisplayMode::Summary {
            self.summary.note_tool_progress(
                parent_name,
                &format!(
                    "{}: {} {status}",
                    t("subtool finished", "子工具结束"),
                    self.summary.display_tool_name(tool_name)
                ),
            );
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
        self.finish_live_tool_status()?;
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
                writeln!(stdout, "{TOOL_BULLET} {}", t("thinking", "思考"))?;
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
        self.finish_live_tool_status()?;
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

    /// 追加写入工具状态事件。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `status`: 工具状态文本
    ///
    /// 返回:
    /// - 写入是否成功
    fn write_tool_event_line(&self, name: &str, status: &str) -> Result<()> {
        let mut stdout = io::stdout();
        writeln!(
            stdout,
            "{}",
            tool_event_text(self.summary.display_tool_name(name), status)
        )?;
        stdout.flush()?;
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

#[cfg(test)]
#[path = "stream_tests.rs"]
mod tests;
