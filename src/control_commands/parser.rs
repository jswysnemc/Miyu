use super::compaction_args::parse_compaction_args;
use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ControlSurface {
    Repl,
    Gateway,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ControlCommand {
    Help,
    New { title: String },
    Compact { keep_tail_turns: usize },
    Clear { all: bool },
    Model { selection: Option<usize> },
}

/// 解析 REPL 或网关控制命令。
///
/// 参数:
/// - `input`: 原始输入文本
/// - `surface`: 命令入口类型
///
/// 返回:
/// - 已识别的控制命令，非控制命令返回空
pub fn parse_control_command(
    input: &str,
    surface: ControlSurface,
) -> Result<Option<ControlCommand>> {
    let Some((name, rest)) = slash_command_parts(input) else {
        return Ok(None);
    };
    let name = name.to_ascii_lowercase();
    if matches_surface_alias(&name, surface, "help", &["帮助"]) {
        return Ok(Some(ControlCommand::Help));
    }
    if matches_surface_alias(&name, surface, "new", &["新建"]) {
        return Ok(Some(ControlCommand::New {
            title: rest.trim().to_string(),
        }));
    }
    if matches_surface_alias(&name, surface, "compact", &["压缩"]) {
        return Ok(Some(ControlCommand::Compact {
            keep_tail_turns: parse_compaction_args(rest)?,
        }));
    }
    if matches_surface_alias(&name, surface, "clear", &["清空"]) {
        return Ok(Some(ControlCommand::Clear {
            all: parse_clear_args(rest)?,
        }));
    }
    if matches_surface_alias(&name, surface, "model", &["模型"]) {
        return Ok(Some(ControlCommand::Model {
            selection: parse_model_args(rest)?,
        }));
    }
    Ok(None)
}

/// 拆分斜杠命令名称和参数。
///
/// 参数:
/// - `input`: 原始输入文本
///
/// 返回:
/// - 命令名和参数文本
fn slash_command_parts(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    let input = input
        .strip_prefix('/')
        .or_else(|| input.strip_prefix('／'))?;
    let command_len = input
        .char_indices()
        .find_map(|(index, ch)| ch.is_whitespace().then_some(index))
        .unwrap_or(input.len());
    let command = &input[..command_len];
    let rest = input[command_len..].trim_start();
    Some((command, rest))
}

/// 判断命令是否命中当前入口可用的别名。
///
/// 参数:
/// - `name`: 已归一化的命令名
/// - `surface`: 命令入口类型
/// - `english`: 英文命令名
/// - `gateway_chinese_aliases`: 网关可用中文别名
///
/// 返回:
/// - 命中时返回 true
fn matches_surface_alias(
    name: &str,
    surface: ControlSurface,
    english: &str,
    gateway_chinese_aliases: &[&str],
) -> bool {
    name == english
        || surface == ControlSurface::Gateway
            && gateway_chinese_aliases.iter().any(|alias| name == *alias)
}

/// 解析清空命令参数。
///
/// 参数:
/// - `input`: 参数文本
///
/// 返回:
/// - 是否清空全部记忆
fn parse_clear_args(input: &str) -> Result<bool> {
    let parts = input.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [] => Ok(false),
        [scope] if scope.eq_ignore_ascii_case("all") || *scope == "全部" => Ok(true),
        [scope] => bail!("unknown clear scope: {scope}"),
        _ => bail!("too many clear arguments"),
    }
}

/// 解析模型命令参数。
///
/// 参数:
/// - `input`: 参数文本
///
/// 返回:
/// - 可选模型序号
fn parse_model_args(input: &str) -> Result<Option<usize>> {
    let parts = input.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [] => Ok(None),
        [index] => {
            Ok(Some(index.parse::<usize>().map_err(|_| {
                anyhow::anyhow!("invalid model index: {index}")
            })?))
        }
        _ => bail!("too many model arguments"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_english_and_chinese_gateway_aliases() {
        assert_eq!(
            parse_control_command("/help", ControlSurface::Gateway).unwrap(),
            Some(ControlCommand::Help)
        );
        assert_eq!(
            parse_control_command("/帮助", ControlSurface::Gateway).unwrap(),
            Some(ControlCommand::Help)
        );
        assert_eq!(
            parse_control_command("/压缩 --keep 3", ControlSurface::Gateway).unwrap(),
            Some(ControlCommand::Compact { keep_tail_turns: 3 })
        );
    }

    #[test]
    fn parses_clear_and_model_arguments() {
        assert_eq!(
            parse_control_command("/清空 全部", ControlSurface::Gateway).unwrap(),
            Some(ControlCommand::Clear { all: true })
        );
        assert_eq!(
            parse_control_command("/模型 2", ControlSurface::Gateway).unwrap(),
            Some(ControlCommand::Model { selection: Some(2) })
        );
    }

    #[test]
    fn repl_does_not_parse_chinese_slash_commands() {
        assert_eq!(
            parse_control_command("/帮助", ControlSurface::Repl).unwrap(),
            None
        );
        assert_eq!(
            parse_control_command("/压缩", ControlSurface::Repl).unwrap(),
            None
        );
    }
}
