/// 工具执行的最终结果。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolOutcome {
    pub(crate) ok: bool,
    pub(crate) output: String,
}

/// 可在 CLI 与 REPL 间共享的工具生命周期模型。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolView {
    pub(crate) name: String,
    pub(crate) arguments: String,
    pub(crate) progress: Option<String>,
    pub(crate) outcome: Option<ToolOutcome>,
}

impl ToolView {
    /// 创建已经收到完整参数的工具调用视图。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `arguments`: 工具参数
    ///
    /// 返回:
    /// - 运行中的工具视图
    pub(crate) fn running(name: String, arguments: String) -> Self {
        Self {
            name,
            arguments,
            progress: None,
            outcome: None,
        }
    }

    /// 创建仅用于参数流预览的工具视图。
    ///
    /// 参数:
    /// - `name`: 工具名称
    /// - `arguments_preview`: 尚未完成的参数文本
    ///
    /// 返回:
    /// - 参数接收中的工具视图
    pub(crate) fn preparing(name: String, arguments_preview: String) -> Self {
        Self::running(name, arguments_preview)
    }

    /// 更新工具进度信息。
    ///
    /// 参数:
    /// - `message`: 最新进度文本
    pub(crate) fn set_progress(&mut self, message: String) {
        self.progress = Some(message);
    }

    /// 完成工具调用。
    ///
    /// 参数:
    /// - `ok`: 工具是否成功
    /// - `output`: 工具输出
    pub(crate) fn finish(&mut self, ok: bool, output: String) {
        self.outcome = Some(ToolOutcome { ok, output });
    }

    /// 判断视图是否属于指定工具且仍在运行。
    ///
    /// 参数:
    /// - `name`: 工具名称
    ///
    /// 返回:
    /// - 是否可以继续更新该视图
    pub(crate) fn is_active_for(&self, name: &str) -> bool {
        self.name == name && self.outcome.is_none()
    }
}
