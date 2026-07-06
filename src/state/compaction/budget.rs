use crate::llm::Usage;

/// 判断 provider 返回的 token 用量是否已经达到自动压缩阈值。
///
/// 参数:
/// - `usage`: provider 返回的最近一次模型用量
/// - `context_tokens`: 当前模型上下文窗口 token 数
/// - `trim_at_ratio`: 自动压缩触发比例
///
/// 返回:
/// - 达到阈值时返回 true
pub fn should_compact_for_usage(usage: &Usage, context_tokens: usize, trim_at_ratio: f32) -> bool {
    if context_tokens == 0 {
        return false;
    }
    let trigger = (context_tokens as f32 * trim_at_ratio).max(1.0) as u64;
    context_usage_tokens(usage) >= trigger
}

/// 返回 provider usage 中可代表当前上下文压力的 token 数。
///
/// 参数:
/// - `usage`: provider 返回的模型用量
///
/// 返回:
/// - 优先使用 total_tokens，缺失时回退到 prompt_tokens + completion_tokens
fn context_usage_tokens(usage: &Usage) -> u64 {
    if usage.total_tokens > 0 {
        usage.total_tokens
    } else {
        usage.prompt_tokens.saturating_add(usage.completion_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_when_context_window_is_missing() {
        let usage = Usage {
            prompt_tokens: 90,
            completion_tokens: 10,
            total_tokens: 100,
        };

        assert!(!should_compact_for_usage(&usage, 0, 0.9));
    }

    #[test]
    fn triggers_when_total_tokens_cross_threshold() {
        let usage = Usage {
            prompt_tokens: 1,
            completion_tokens: 1,
            total_tokens: 91,
        };

        assert!(should_compact_for_usage(&usage, 100, 0.9));
    }

    #[test]
    fn skips_when_total_tokens_are_under_threshold() {
        let usage = Usage {
            prompt_tokens: 1,
            completion_tokens: 1,
            total_tokens: 89,
        };

        assert!(!should_compact_for_usage(&usage, 100, 0.9));
    }

    #[test]
    fn falls_back_to_prompt_and_completion_tokens() {
        let usage = Usage {
            prompt_tokens: 80,
            completion_tokens: 10,
            total_tokens: 0,
        };

        assert!(should_compact_for_usage(&usage, 100, 0.9));
    }
}
