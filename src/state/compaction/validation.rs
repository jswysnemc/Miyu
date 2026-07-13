use anyhow::{bail, Result};

const REQUIRED_HEADINGS: &[&str] = &[
    "## Goal",
    "## Constraints & Preferences",
    "## Progress",
    "### Done",
    "### In Progress",
    "### Blocked",
    "## Key Decisions",
    "## Next Steps",
    "## Critical Context",
    "## Relevant Files",
];

/// 计算压缩摘要允许占用的最大字符数。
///
/// 参数:
/// - `context_limit_chars`: 模型上下文字符预算
///
/// 返回:
/// - 摘要最大字符数
pub(crate) fn summary_char_limit(context_limit_chars: usize) -> usize {
    ((context_limit_chars as f32 * 0.15) as usize).clamp(512, 30_000)
}

/// 校验压缩摘要结构、顺序与体积。
///
/// 参数:
/// - `summary`: 模型生成的摘要
/// - `max_chars`: 摘要最大字符数
///
/// 返回:
/// - 摘要满足持久化约束时成功
pub(crate) fn validate_summary(summary: &str, max_chars: usize) -> Result<()> {
    let summary = summary.trim();
    if summary.is_empty() {
        bail!("compaction summary is empty")
    }
    let chars = summary.chars().count();
    if chars > max_chars {
        bail!("compaction summary exceeds size limit: summary_chars={chars}, max_chars={max_chars}")
    }
    let mut cursor = 0usize;
    for heading in REQUIRED_HEADINGS {
        let Some(offset) = summary[cursor..].find(heading) else {
            bail!("compaction summary is missing required heading: {heading}")
        };
        cursor += offset + heading.len();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unstructured_summary() {
        assert!(validate_summary("plain summary", 10_000).is_err());
    }

    #[test]
    fn accepts_complete_summary_structure() {
        let summary = REQUIRED_HEADINGS.join("\n- item\n");
        assert!(validate_summary(&summary, 10_000).is_ok());
    }
}
