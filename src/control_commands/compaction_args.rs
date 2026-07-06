use anyhow::{bail, Result};

pub const DEFAULT_KEEP_TAIL_TURNS: usize = 2;

/// 解析手动压缩参数。
///
/// 参数:
/// - `input`: 命令参数文本
///
/// 返回:
/// - 需要保留的最近非运行轮次数量
pub fn parse_compaction_args(input: &str) -> Result<usize> {
    let mut keep_tail_turns = DEFAULT_KEEP_TAIL_TURNS;
    let parts = input.split_whitespace().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < parts.len() {
        let part = parts[index];
        if matches!(part, "--keep" | "-k" | "--保留" | "保留") {
            index += 1;
            let Some(value) = parts.get(index) else {
                bail!("missing value for {part}");
            };
            keep_tail_turns = parse_keep_tail_turns(value)?;
        } else if let Some(value) = part
            .strip_prefix("--keep=")
            .or_else(|| part.strip_prefix("keep="))
            .or_else(|| part.strip_prefix("--保留="))
            .or_else(|| part.strip_prefix("保留="))
        {
            keep_tail_turns = parse_keep_tail_turns(value)?;
        } else if part.chars().all(|ch| ch.is_ascii_digit()) {
            keep_tail_turns = parse_keep_tail_turns(part)?;
        } else {
            bail!("unknown compaction argument: {part}");
        }
        index += 1;
    }
    Ok(keep_tail_turns)
}

/// 解析保留轮次数量。
///
/// 参数:
/// - `value`: 原始数量文本
///
/// 返回:
/// - 保留轮次数量
fn parse_keep_tail_turns(value: &str) -> Result<usize> {
    value
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("invalid keep value: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_keep_value() {
        assert_eq!(parse_compaction_args("").unwrap(), DEFAULT_KEEP_TAIL_TURNS);
    }

    #[test]
    fn parses_english_and_chinese_keep_aliases() {
        assert_eq!(parse_compaction_args("--keep 3").unwrap(), 3);
        assert_eq!(parse_compaction_args("-k 4").unwrap(), 4);
        assert_eq!(parse_compaction_args("保留 5").unwrap(), 5);
        assert_eq!(parse_compaction_args("--保留=6").unwrap(), 6);
    }
}
