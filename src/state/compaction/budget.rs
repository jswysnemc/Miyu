/// 判断当前上下文估算是否已经达到自动压缩阈值。
///
/// 参数:
/// - `context_chars`: 当前请求上下文估算字符数
/// - `context_limit_chars`: 当前模型上下文窗口字符数
/// - `trim_at_ratio`: 自动压缩触发比例
///
/// 返回:
/// - 达到阈值时返回 true
pub fn should_compact_for_context_chars(
    context_chars: usize,
    context_limit_chars: usize,
    trim_at_ratio: f32,
) -> bool {
    if context_limit_chars == 0 {
        return false;
    }
    context_chars >= trigger_chars(context_limit_chars, trim_at_ratio)
}

/// 计算自动压缩触发阈值。
///
/// 参数:
/// - `context_limit_chars`: 当前模型上下文窗口字符数
/// - `trim_at_ratio`: 自动压缩触发比例
///
/// 返回:
/// - 触发压缩的字符数
fn trigger_chars(context_limit_chars: usize, trim_at_ratio: f32) -> usize {
    ((context_limit_chars as f32) * trim_at_ratio).max(1.0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triggers_when_context_chars_cross_threshold() {
        assert!(should_compact_for_context_chars(90, 100, 0.9));
    }

    #[test]
    fn skips_when_context_chars_are_under_threshold() {
        assert!(!should_compact_for_context_chars(89, 100, 0.9));
    }
}
