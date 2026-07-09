use crate::render::streaming_replace::{clear_rendered_rows, raw_visual_rows};

/// Markdown 资产块流式替换状态。
pub(crate) struct StreamingAssetBlock {
    raw_visual_rows: usize,
}

impl StreamingAssetBlock {
    /// 创建资产块流式替换状态。
    ///
    /// 返回:
    /// - 新的资产块替换状态
    pub(crate) fn new() -> Self {
        Self { raw_visual_rows: 0 }
    }

    /// 推入一行资产块原始文本。
    ///
    /// 参数:
    /// - `line`: 当前收到的 Markdown 原始行
    ///
    /// 返回:
    /// - 需要立即写入终端的原始 Markdown 行
    pub(crate) fn push_line(&mut self, line: &str) -> String {
        self.raw_visual_rows += raw_visual_rows(line);
        format!("{line}\n")
    }

    /// 结束资产块并用最终渲染结果替换原始文本。
    ///
    /// 参数:
    /// - `rendered`: 图片渲染文本或错误提示
    ///
    /// 返回:
    /// - 清除原文后的最终渲染文本
    pub(crate) fn finish(&mut self, rendered: String) -> String {
        // 1. 先清除流式阶段已经输出的原始 Markdown
        let mut output = clear_rendered_rows(self.raw_visual_rows);
        // 2. 再输出最终资产渲染结果
        output.push_str(&rendered);
        self.raw_visual_rows = 0;
        output
    }

    /// 重置资产块替换状态。
    pub(crate) fn reset(&mut self) {
        self.raw_visual_rows = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_streamed_asset_source_with_rendered_output() {
        let mut block = StreamingAssetBlock::new();

        assert_eq!(block.push_line("```mermaid"), "```mermaid\n");
        assert_eq!(block.push_line("graph TD"), "graph TD\n");
        let output = block.finish("[diagram]\n".to_string());

        assert!(output.starts_with("\x1b[1A\r\x1b[2K"));
        assert!(output.ends_with("[diagram]\n"));
    }
}
