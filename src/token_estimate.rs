//! 粗略 token 估算：区分 CJK 与拉丁字符，避免中文场景下 `chars/4` 严重偏松。

const CHARS_PER_TOKEN_LATIN: usize = 4;
const CHARS_PER_TOKEN_CJK: usize = 2;

/// 估算单段文本的 token 数（至少为 1，空串为 0）。
///
/// 参数:
/// - `text`: 待估算文本
///
/// 返回:
/// - 粗略 token 数
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    text_tokens(text).max(1)
}

/// 估算多段文本合计 token 数。
///
/// 参数:
/// - `texts`: 文本片段
///
/// 返回:
/// - 粗略 token 数（u64）
pub fn estimate_texts_tokens(texts: &[&str]) -> u64 {
    let combined: String = texts.iter().copied().collect();
    estimate_tokens(&combined) as u64
}

/// 将 token 预算换算为保守字符容量。
///
/// 参数:
/// - `tokens`: token 预算
///
/// 返回:
/// - 任意 CJK / 拉丁混排下都不会超出 token 预算的字符数
pub fn conservative_char_capacity(tokens: usize) -> usize {
    tokens.saturating_mul(CHARS_PER_TOKEN_CJK)
}

/// 按 CJK / 拉丁分别折算，不做 min=1。
///
/// 参数:
/// - `text`: 待估算文本
///
/// 返回:
/// - 未钳制的 token 估算
fn text_tokens(text: &str) -> usize {
    let mut cjk = 0usize;
    let mut latin = 0usize;
    for ch in text.chars() {
        if is_cjk(ch) {
            cjk += 1;
        } else {
            latin += 1;
        }
    }
    // 分组向上取整，与原先 chars.div_ceil(4) 在纯拉丁文本上对齐
    cjk.div_ceil(CHARS_PER_TOKEN_CJK) + latin.div_ceil(CHARS_PER_TOKEN_LATIN)
}

/// 判断字符是否按 CJK 密度估算。
///
/// 参数:
/// - `ch`: 字符
///
/// 返回:
/// - 是否为 CJK / 假名 / 韩文 / 全角
fn is_cjk(ch: char) -> bool {
    let code = ch as u32;
    (0x4E00..=0x9FFF).contains(&code) // CJK Unified Ideographs
        || (0x3400..=0x4DBF).contains(&code) // CJK Extension A
        || (0x20000..=0x2A6DF).contains(&code) // CJK Extension B
        || (0x3040..=0x30FF).contains(&code) // Hiragana + Katakana
        || (0xAC00..=0xD7AF).contains(&code) // Hangul Syllables
        || (0xFF00..=0xFFEF).contains(&code) // Fullwidth Forms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin_uses_four_chars_per_token() {
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn cjk_uses_two_chars_per_token() {
        assert_eq!(estimate_tokens("你好"), 1);
        assert_eq!(estimate_tokens("你好世界"), 2);
    }

    #[test]
    fn mixed_text_combines_rates() {
        // 4 拉丁 + 2 汉字 => 1 + 1
        assert_eq!(estimate_tokens("abcd你好"), 2);
    }

    #[test]
    fn empty_is_zero() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_texts_tokens(&[]), 0);
    }

    #[test]
    fn conservative_capacity_never_exceeds_token_budget() {
        // 全 CJK 是最坏情况：2 字符/token，容量按此换算保证任何混排都不超预算
        let budget = conservative_char_capacity(1_000);
        assert_eq!(budget, 2_000);
        let cjk_text = "你".repeat(budget);
        assert!(estimate_tokens(&cjk_text) <= 1_000);
        let latin_text = "a".repeat(budget);
        assert!(estimate_tokens(&latin_text) <= 1_000);
    }
}
