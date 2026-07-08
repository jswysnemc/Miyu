use super::Agent;
use crate::llm::ToolCall;
use anyhow::Result;

impl Agent {
    /// 记录工具调用开始事件。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮次标识
    /// - `seq`: 当前轮内工具调用顺序
    /// - `call`: provider 工具调用
    ///
    /// 返回:
    /// - 写入是否成功
    pub(super) fn record_tool_call_started(
        &self,
        turn_id: &str,
        seq: usize,
        call: &ToolCall,
    ) -> Result<()> {
        self.state.record_tool_call_started(
            turn_id,
            seq,
            &call.id,
            &call.function.name,
            &call.function.arguments,
        )
    }

    /// 记录工具调用结果事件。
    ///
    /// 参数:
    /// - `turn_id`: 当前轮次标识
    /// - `call`: provider 工具调用
    /// - `ok`: 工具是否成功
    /// - `raw_output`: 原始工具输出
    /// - `context_output`: 模型可见工具输出
    ///
    /// 返回:
    /// - 写入是否成功
    pub(super) fn record_tool_result_completed(
        &self,
        turn_id: &str,
        call: &ToolCall,
        ok: bool,
        raw_output: &str,
        context_output: &str,
    ) -> Result<()> {
        let error = (!ok).then_some(raw_output);
        let result_ref = self.state.save_clipped_tool_output_replacement(
            &call.id,
            raw_output,
            context_output,
        )?;
        self.state.record_tool_result_completed(
            turn_id,
            &call.id,
            ok,
            context_output,
            result_ref.as_deref(),
            error,
            raw_output.chars().count(),
        )
    }
}
