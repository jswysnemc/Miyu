use super::StateStore;
use anyhow::Result;

pub struct PendingTurnGuard {
    state: StateStore,
    turn_id: String,
    completed: bool,
}

impl PendingTurnGuard {
    /// 创建待完成轮次守卫。
    ///
    /// 参数:
    /// - `state`: 状态存储
    /// - `turn_id`: 当前轮唯一标识
    ///
    /// 返回:
    /// - 待完成轮次守卫
    pub fn new(state: StateStore, turn_id: String) -> Self {
        Self {
            state,
            turn_id,
            completed: false,
        }
    }

    /// 完成当前轮次并关闭守卫。
    ///
    /// 参数:
    /// - `content`: 助手回复
    /// - `reasoning`: 可选推理内容
    ///
    /// 返回:
    /// - 完成是否成功
    pub fn complete(mut self, content: &str, reasoning: Option<&str>) -> Result<()> {
        self.state
            .complete_turn(&self.turn_id, content, reasoning)?;
        self.completed = true;
        Ok(())
    }

    /// 手动中断当前轮次。
    ///
    /// 返回:
    /// - 中断是否成功
    #[allow(dead_code)]
    pub fn interrupt(&mut self) -> Result<()> {
        if !self.completed {
            self.state.interrupt_turn(&self.turn_id)?;
            self.completed = true;
        }
        Ok(())
    }
}

impl Drop for PendingTurnGuard {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self.state.interrupt_turn(&self.turn_id);
        }
    }
}
