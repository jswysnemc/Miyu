use super::cell::{HistoryCell, TranscriptMode};
use super::line::AnsiLine;
use super::markdown_cell;
use super::reasoning_cell;
use super::tool_cell::ToolCell;
use super::welcome_cell::WelcomeCell;
use crate::llm::{ChatStreamChunk, ChatStreamKind, ToolCallStreamProgress};
use crate::render::tool_view::ToolView;
use crate::render::work_status::WorkStatus;
use crate::render::{ReasoningDisplayMode, ToolCallDisplayMode};
use std::collections::VecDeque;

/// REPL transcript 的渲染选项快照。
#[derive(Clone, Copy, Debug)]
pub(crate) struct TranscriptRenderOptions {
    pub(crate) reasoning_mode: ReasoningDisplayMode,
    pub(crate) tool_call_mode: ToolCallDisplayMode,
}

/// 仍在生成中的文本 source。
#[derive(Clone, Debug, Eq, PartialEq)]
struct LiveTail {
    kind: ChatStreamKind,
    source: String,
}

/// 正在接收参数的工具调用预览。
#[derive(Clone, Debug, Eq, PartialEq)]
struct LiveToolCall {
    name: String,
    arguments_preview: String,
}

impl LiveTail {
    /// 将临时流式文本转换为可重放 history cell。
    ///
    /// 参数:
    /// - `self`: 当前临时流式文本
    ///
    /// 返回:
    /// - 对应的历史 cell
    fn into_cell(self) -> HistoryCell {
        match self.kind {
            ChatStreamKind::Content => HistoryCell::markdown(self.source),
            ChatStreamKind::Reasoning => HistoryCell::reasoning(self.source),
        }
    }
}

/// 保存 REPL 会话的定稿 cell 与可变流式尾部。
pub(crate) struct TranscriptStore {
    cells: Vec<HistoryCell>,
    live_tail: Option<LiveTail>,
    live_tool_call: Option<LiveToolCall>,
    live_animation_frame: usize,
    active_tool_index: Option<usize>,
    work_status: Option<WorkStatus>,
    row_cap: usize,
}

impl TranscriptStore {
    /// 创建空 transcript。
    ///
    /// 参数:
    /// - `row_cap`: resize 重放时保留的最大视觉行数
    ///
    /// 返回:
    /// - 空 transcript
    pub(crate) fn new(row_cap: usize) -> Self {
        Self {
            cells: Vec::new(),
            live_tail: None,
            live_tool_call: None,
            live_animation_frame: 0,
            active_tool_index: None,
            work_status: None,
            row_cap: row_cap.max(1),
        }
    }

    /// 更新 row cap。
    ///
    /// 参数:
    /// - `row_cap`: resize 重放时保留的最大视觉行数
    ///
    /// 返回:
    /// - 无
    pub(crate) fn set_row_cap(&mut self, row_cap: usize) {
        self.row_cap = row_cap.max(1);
    }

    /// 记录用户输入回显。
    ///
    /// 参数:
    /// - `mode`: 用户提交时的 REPL 模式
    /// - `text`: 原始输入文本
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_user_echo(&mut self, mode: TranscriptMode, text: String) {
        self.push_cell(HistoryCell::user_echo(mode, text));
    }

    /// 记录系统提示或控制命令输出。
    ///
    /// 参数:
    /// - `text`: 原始消息文本
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_meta(&mut self, text: String) {
        self.push_cell(HistoryCell::meta(text));
    }

    /// 记录 REPL 启动信息面板。
    ///
    /// 参数:
    /// - `cell`: 启动信息 source
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_welcome(&mut self, cell: WelcomeCell) {
        self.push_cell(HistoryCell::welcome(cell));
    }

    /// 记录模型流式文本片段，并在种类变化时收敛旧尾部。
    ///
    /// 参数:
    /// - `chunk`: 模型流式文本片段
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_chunk(&mut self, chunk: &ChatStreamChunk) {
        match self.live_tail.as_mut() {
            Some(tail) if tail.kind == chunk.kind => tail.source.push_str(&chunk.text),
            Some(_) => {
                self.finalize_live_tail();
                self.live_animation_frame = 0;
                self.live_tail = Some(LiveTail {
                    kind: chunk.kind,
                    source: chunk.text.clone(),
                });
            }
            None => {
                self.live_animation_frame = 0;
                self.live_tail = Some(LiveTail {
                    kind: chunk.kind,
                    source: chunk.text.clone(),
                });
            }
        }
    }

    /// 记录尚未定稿的工具参数流预览。
    ///
    /// 参数:
    /// - `progress`: 当前工具调用参数接收进度
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_tool_call_progress(&mut self, progress: &ToolCallStreamProgress) {
        self.live_tool_call = Some(LiveToolCall {
            name: progress.name.clone().unwrap_or_else(|| "tool".to_string()),
            arguments_preview: progress.arguments_preview.clone(),
        });
    }

    /// 更新当前单轮工作状态。
    ///
    /// 参数:
    /// - `status`: 新工作状态
    ///
    /// 返回:
    /// - 状态是否发生变化
    pub(crate) fn set_work_status(&mut self, status: WorkStatus) -> bool {
        if self.work_status == Some(status) {
            return false;
        }
        self.work_status = Some(status);
        true
    }

    /// 清除当前单轮工作状态。
    ///
    /// 返回:
    /// - 是否清除了状态
    pub(crate) fn clear_work_status(&mut self) -> bool {
        self.work_status.take().is_some()
    }

    /// 在追加定稿 cell 前收敛当前流式尾部。
    ///
    /// 参数:
    /// - `cell`: 定稿 history cell
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_cell(&mut self, cell: HistoryCell) {
        self.finalize_live_tail();
        self.cells.push(cell);
    }

    /// 记录工具调用。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `arguments`: 原始工具参数
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_tool_call(&mut self, name: String, arguments: String) {
        self.finalize_live_tail();
        self.live_tool_call = None;
        if name == "edit_file" {
            self.active_tool_index = None;
            self.push_cell(HistoryCell::diff(arguments));
        } else {
            let index = self.cells.len();
            self.cells
                .push(HistoryCell::Tool(ToolCell::Invocation(ToolView::running(
                    name, arguments,
                ))));
            self.active_tool_index = Some(index);
        }
    }

    /// 记录工具结果。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `ok`: 工具是否成功
    /// - `output`: 原始工具输出
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_tool_result(&mut self, name: String, ok: bool, output: String) {
        self.finalize_live_tail();
        if self.update_active_tool(&name, |view| view.finish(ok, output.clone())) {
            self.active_tool_index = None;
            return;
        }
        let mut view = ToolView::running(name, String::new());
        view.finish(ok, output);
        self.push_cell(HistoryCell::Tool(ToolCell::Invocation(view)));
        self.active_tool_index = None;
        self.work_status = None;
    }

    /// 记录工具进度。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `message`: 原始进度信息
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_tool_progress(&mut self, name: String, message: String) {
        self.finalize_live_tail();
        if self.update_active_tool(&name, |view| view.set_progress(message.clone())) {
            return;
        }
        let mut view = ToolView::running(name, String::new());
        view.set_progress(message);
        self.push_cell(HistoryCell::Tool(ToolCell::Invocation(view)));
    }

    /// 记录上下文压缩开始事件。
    ///
    /// 参数:
    /// - `turn_count`: 待压缩的轮次数
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_compaction_started(&mut self, turn_count: usize) {
        self.active_tool_index = None;
        self.push_cell(HistoryCell::Tool(ToolCell::CompactionStarted {
            turn_count,
        }));
    }

    /// 记录上下文压缩结束事件。
    ///
    /// 参数:
    /// - `applied`: 是否成功应用压缩
    ///
    /// 返回:
    /// - 无
    pub(crate) fn push_compaction_finished(&mut self, applied: bool) {
        self.active_tool_index = None;
        self.push_cell(HistoryCell::Tool(ToolCell::CompactionFinished { applied }));
    }

    /// 将当前流式尾部收敛为定稿 cell。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 是否发生收敛
    pub(crate) fn finalize_live_tail(&mut self) -> bool {
        let cleared_tool_preview = self.live_tool_call.take().is_some();
        let Some(tail) = self.live_tail.take() else {
            return cleared_tool_preview;
        };
        if tail.source.is_empty() {
            return cleared_tool_preview;
        }
        self.cells.push(tail.into_cell());
        true
    }

    /// 清空当前 REPL 会话的所有 source cell。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 无
    pub(crate) fn clear(&mut self) {
        self.cells.clear();
        self.live_tail = None;
        self.live_tool_call = None;
        self.live_animation_frame = 0;
        self.active_tool_index = None;
    }

    /// 更新当前活动工具单元。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `update`: 工具视图更新函数
    ///
    /// 返回:
    /// - 是否找到可更新的活动工具
    fn update_active_tool<F>(&mut self, name: &str, update: F) -> bool
    where
        F: FnOnce(&mut ToolView),
    {
        let Some(index) = self.active_tool_index else {
            return false;
        };
        let Some(HistoryCell::Tool(ToolCell::Invocation(view))) = self.cells.get_mut(index) else {
            return false;
        };
        if !view.is_active_for(name) {
            return false;
        }
        update(view);
        true
    }

    /// 推进 live reasoning 的跳动帧。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 是否存在需要刷新的 reasoning live tail
    pub(crate) fn advance_live_animation(&mut self) -> bool {
        let Some(tail) = &self.live_tail else {
            return false;
        };
        if tail.kind != ChatStreamKind::Reasoning || tail.source.is_empty() {
            return false;
        }
        self.live_animation_frame = self.live_animation_frame.wrapping_add(1);
        true
    }

    /// 渲染所有定稿 cell 与当前 live tail 的尾部窗口。
    ///
    /// 参数:
    /// - `width`: 当前终端列数
    /// - `options`: transcript 渲染选项
    ///
    /// 返回:
    /// - row cap 范围内的预换行 ANSI 行
    pub(crate) fn display_tail(
        &self,
        width: usize,
        options: &TranscriptRenderOptions,
    ) -> Vec<AnsiLine> {
        let mut retained = VecDeque::new();

        // 1. 流式重放只恢复已提交的稳定行，未结束尾部等待收敛后再写入
        prepend_tail_lines(
            &mut retained,
            self.display_live_tail(width, options),
            self.row_cap,
        );

        // 2. 从最新定稿 cell 向前渲染，到达 row cap 后停止处理更早 source
        for cell in self.cells.iter().rev() {
            if retained.len() >= self.row_cap {
                break;
            }
            prepend_tail_lines(
                &mut retained,
                cell.display_lines(width, options),
                self.row_cap,
            );
        }
        retained.into_iter().collect()
    }

    /// 渲染当前 live tail，用于流式增量插入。
    ///
    /// 参数:
    /// - `width`: 当前终端列数
    /// - `options`: transcript 渲染选项
    ///
    /// 返回:
    /// - 当前 live tail 的预换行 ANSI 行
    pub(crate) fn display_live_tail(
        &self,
        width: usize,
        options: &TranscriptRenderOptions,
    ) -> Vec<AnsiLine> {
        let mut lines = Vec::new();
        if let Some(status) = self.work_status {
            lines.extend(AnsiLine::wrap_block(
                &super::work_status_cell::render(status),
                width,
            ));
        }
        if let Some(tail) = &self.live_tail {
            let rendered = match tail.kind {
                ChatStreamKind::Content => markdown_cell::render_completed(&tail.source),
                // reasoning 在定稿前显示节流的字符计数与跳动标记，结束后再按配置完整固化
                ChatStreamKind::Reasoning => reasoning_cell::render_live(
                    &tail.source,
                    options.reasoning_mode,
                    self.live_animation_frame,
                ),
            };
            if !rendered.is_empty() {
                lines.extend(AnsiLine::wrap_block(&rendered, width));
            }
        }
        if let Some(tool_call) = &self.live_tool_call {
            let rendered = super::tool_cell::render_live_call(
                &tool_call.name,
                &tool_call.arguments_preview,
                options.tool_call_mode,
            );
            if !rendered.is_empty() {
                lines.extend(AnsiLine::wrap_block(&rendered, width));
            }
        }
        lines
    }
}

/// 从一组预换行行中保留尾部，并将其插入当前尾部窗口前方。
fn prepend_tail_lines(retained: &mut VecDeque<AnsiLine>, lines: Vec<AnsiLine>, row_cap: usize) {
    for line in lines.into_iter().rev() {
        retained.push_front(line);
        if retained.len() > row_cap {
            retained.pop_back();
        }
    }
}
