use crate::i18n::text as t;
use anyhow::{bail, Result};

/// 禁止用 Shell 命令代替专用文件写入工具。
///
/// 参数:
/// - `command`: Shell 命令文本
///
/// 返回:
/// - 命令未写入实际文件时返回成功
pub(super) fn ensure_not_file_write_command(command: &str) -> Result<()> {
    if has_file_output_redirection(command) || has_tee_write(command) || has_cat_heredoc(command) {
        bail!(
            "{}",
            t(
                "Use edit_file for file writes instead of shell file writes",
                "请使用 edit_file 写文件，不要用 shell 写文件"
            )
        );
    }
    Ok(())
}

/// 判断命令是否把任意文件描述符重定向到实际文件。
///
/// 参数:
/// - `command`: Shell 命令文本
///
/// 返回:
/// - 存在实际文件写入目标时返回 true
fn has_file_output_redirection(command: &str) -> bool {
    let chars = command.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut single = false;
    let mut double = false;
    let mut escaped = false;

    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if ch == '\'' && !double {
            single = !single;
            index += 1;
            continue;
        }
        if ch == '"' && !single {
            double = !double;
            index += 1;
            continue;
        }
        if ch != '>' || single || double {
            index += 1;
            continue;
        }

        // 1. 跳过追加重定向符号和操作符后的空白
        index += 1;
        if chars.get(index) == Some(&'>') {
            index += 1;
        }
        skip_whitespace(&chars, &mut index);

        // 2. 文件描述符复制或关闭不写入文件
        if chars.get(index) == Some(&'&') {
            index += 1;
            skip_whitespace(&chars, &mut index);
            let start = index;
            while chars.get(index).is_some_and(|value| value.is_ascii_digit()) {
                index += 1;
            }
            let closes_descriptor = chars.get(index) == Some(&'-');
            if closes_descriptor {
                index += 1;
            }
            if (index > start) && is_shell_token_boundary(chars.get(index).copied()) {
                continue;
            }
            return true;
        }

        // 3. 空设备允许丢弃输出，其他目标均视为文件写入
        let target = read_redirection_target(&chars, &mut index);
        if !is_null_device(&target) {
            return true;
        }
    }
    false
}

/// 判断字符是否位于 Shell 词边界。
///
/// 参数:
/// - `value`: 待判断字符，空值表示命令结束
///
/// 返回:
/// - 命令结束、空白或控制符返回 true
fn is_shell_token_boundary(value: Option<char>) -> bool {
    value.is_none_or(|ch| {
        ch.is_whitespace() || matches!(ch, '|' | ';' | '&' | '(' | ')' | '<' | '>')
    })
}

/// 跳过字符列表中的连续空白。
///
/// 参数:
/// - `chars`: 命令字符列表
/// - `index`: 当前读取位置
///
/// 返回:
/// - 无返回值，读取位置会移动到下一个非空白字符
fn skip_whitespace(chars: &[char], index: &mut usize) {
    while chars.get(*index).is_some_and(|value| value.is_whitespace()) {
        *index += 1;
    }
}

/// 读取输出重定向目标并移除包裹引号。
///
/// 参数:
/// - `chars`: 命令字符列表
/// - `index`: 目标起始位置
///
/// 返回:
/// - 重定向目标文本
fn read_redirection_target(chars: &[char], index: &mut usize) -> String {
    let quote = chars
        .get(*index)
        .copied()
        .filter(|value| matches!(value, '\'' | '"'));
    if quote.is_some() {
        *index += 1;
    }
    let start = *index;
    while let Some(ch) = chars.get(*index) {
        if quote.is_some_and(|value| value == *ch)
            || quote.is_none()
                && (ch.is_whitespace() || matches!(ch, '|' | ';' | '&' | '(' | ')' | '<' | '>'))
        {
            break;
        }
        *index += 1;
    }
    let target = chars[start..*index].iter().collect::<String>();
    if quote.is_some() && chars.get(*index) == quote.as_ref() {
        *index += 1;
    }
    target
}

/// 判断重定向目标是否为跨平台空设备。
///
/// 参数:
/// - `target`: 重定向目标文本
///
/// 返回:
/// - `/dev/null` 或 `NUL` 返回 true
fn is_null_device(target: &str) -> bool {
    target == "/dev/null" || target.eq_ignore_ascii_case("nul")
}

/// 判断命令是否通过 tee 写入文件。
///
/// 参数:
/// - `command`: Shell 命令文本
///
/// 返回:
/// - tee 包含文件目标时返回 true
fn has_tee_write(command: &str) -> bool {
    let words = shell_words(command);
    words
        .iter()
        .enumerate()
        .any(|(index, word)| is_tee_command(word) && has_tee_target(&words[index + 1..]))
}

/// 判断 Shell 词是否是 tee 命令。
///
/// 参数:
/// - `word`: Shell 词
///
/// 返回:
/// - tee 命令返回 true
fn is_tee_command(word: &str) -> bool {
    word == "tee" || word.ends_with("/tee")
}

/// 判断 tee 后续参数是否包含文件目标。
///
/// 参数:
/// - `words`: tee 后面的 Shell 词
///
/// 返回:
/// - 包含文件目标时返回 true
fn has_tee_target(words: &[String]) -> bool {
    words
        .iter()
        .any(|word| word != "-" && !word.starts_with('-'))
}

/// 判断命令是否使用 cat heredoc 写入文件。
///
/// 参数:
/// - `command`: Shell 命令文本
///
/// 返回:
/// - cat heredoc 返回 true
fn has_cat_heredoc(command: &str) -> bool {
    command.contains("<<")
        && shell_words(command)
            .iter()
            .any(|word| word == "cat" || word.ends_with("/cat"))
}

/// 提取未加引号的 Shell 词。
///
/// 参数:
/// - `command`: Shell 命令文本
///
/// 返回:
/// - 简化后的词列表
fn shell_words(command: &str) -> Vec<String> {
    command
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '|' | ';' | '&' | '(' | ')'))
        .map(|word| word.trim_matches(|ch: char| matches!(ch, '"' | '\'')))
        .filter(|word| !word.is_empty())
        .map(|word| word.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_null_devices_and_file_descriptor_duplication() {
        assert!(ensure_not_file_write_command("grep foo file >/dev/null").is_ok());
        assert!(ensure_not_file_write_command("grep foo file > /dev/null").is_ok());
        assert!(ensure_not_file_write_command("grep foo file 1>/dev/null").is_ok());
        assert!(ensure_not_file_write_command("grep foo file 2>/dev/null").is_ok());
        assert!(ensure_not_file_write_command("Write-Output hello > NUL").is_ok());
        assert!(ensure_not_file_write_command("grep foo file 2>&1").is_ok());
        assert!(ensure_not_file_write_command("grep foo file >&2").is_ok());
        assert!(ensure_not_file_write_command("grep foo file 2>&-").is_ok());
        assert!(ensure_not_file_write_command("grep foo file 2>&1file").is_err());
    }

    #[test]
    fn rejects_every_file_descriptor_redirected_to_files() {
        assert!(ensure_not_file_write_command("printf hello > result.txt").is_err());
        assert!(ensure_not_file_write_command("printf hello 1>result.txt").is_err());
        assert!(ensure_not_file_write_command("printf hello 2>error.log").is_err());
        assert!(ensure_not_file_write_command("printf hello >> result.txt").is_err());
        assert!(ensure_not_file_write_command("printf hello 2>>error.log").is_err());
        assert!(ensure_not_file_write_command("printf hello >&combined.log").is_err());
    }

    #[test]
    fn preserves_existing_shell_file_write_guards() {
        assert!(ensure_not_file_write_command("cat <<'EOF' > file\nx\nEOF").is_err());
        assert!(ensure_not_file_write_command("printf hello | tee file").is_err());
        assert!(ensure_not_file_write_command("tee --help").is_ok());
        assert!(ensure_not_file_write_command("printf 'a > b'").is_ok());
    }

    #[test]
    fn allows_drawio_validation_fallback_command() {
        let command = "if command -v drawio >/dev/null; then drawio -x -f png -e -b 10 -o elephant-close-fridge.drawio.png elephant-close-fridge.drawio; elif command -v xmllint >/dev/null; then xmllint --noout elephant-close-fridge.drawio; else python3 -c \"import xml.etree.ElementTree as ET; ET.parse('elephant-close-fridge.drawio')\"; fi\nif command -v drawio >/dev/null; then drawio -x -f png -e -b 10 -o elephant-close-fridge.drawio.png elephant-close-fridge.drawio; elif command -v xmllint >/dev/null; then xmllint --noout elephant-close-fridge.drawio; else python3 -c \"import xml.etree.ElementTree as ET; ET.parse('elephant-close-fridge.drawio')\"; fi";
        assert!(ensure_not_file_write_command(command).is_ok());
    }
}
