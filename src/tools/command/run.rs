use super::process::run_shell_command;
use crate::i18n::text as t;
use crate::tools::{ToolRegistry, ToolSpec};
use anyhow::{bail, Result};
use serde_json::{json, Value};

const MAX_COMMAND_OUTPUT_CHARS: usize = 20_000;

/// 注册可写命令执行工具。
///
/// 参数:
/// - `registry`: 工具注册表
/// - `allow_command_execution`: 是否允许执行命令
pub(crate) fn register(registry: &mut ToolRegistry, allow_command_execution: bool) {
    registry.register(ToolSpec::new(
        "run_command",
        t("Run a shell command in the workspace when skills.allow_command_execution is enabled. Do not use this to create or modify files; use write_file or edit_file instead.", "当 skills.allow_command_execution 启用时，在工作区运行 shell 命令。不要用它创建或修改文件，应使用 write_file 或 edit_file。"),
        json!({"type":"object","properties":{"command":{"type":"string","description": t("Command to run.", "要运行的命令。")},"timeout_seconds":{"type":"integer","description": t("Optional timeout in seconds.", "可选超时时间，单位秒。")}},"required":["command"],"additionalProperties":false}),
        move |args| async move { run_command(args, allow_command_execution).await },
    ).writes());
}

/// 注册只读命令执行工具。
///
/// 参数:
/// - `registry`: 工具注册表
pub(crate) fn register_readonly(registry: &mut ToolRegistry) {
    registry.register(ToolSpec::new(
        "run_command",
        t("Run an explicitly read-only shell command for inspection. Mutating commands are blocked in plan mode.", "运行明确只读的 shell 命令用于检查。计划模式会阻止修改性命令。"),
        json!({"type":"object","properties":{"command":{"type":"string","description": t("Read-only command to run.", "要运行的只读命令。")},"timeout_seconds":{"type":"integer","description": t("Optional timeout in seconds.", "可选超时时间，单位秒。")}},"required":["command"],"additionalProperties":false}),
        |args| async move { run_readonly_command(args).await },
    ));
}

/// 执行普通 shell 命令。
///
/// 参数:
/// - `args`: 工具参数
/// - `allowed`: 是否允许命令执行
///
/// 返回:
/// - JSON 格式命令结果
async fn run_command(args: Value, allowed: bool) -> Result<String> {
    if !allowed {
        bail!("{}", t("command execution is disabled; set skills.allow_command_execution=true in config.jsonc to enable run_command", "命令执行已禁用；请在 config.jsonc 中设置 skills.allow_command_execution=true 以启用 run_command"));
    }
    let command = required(&args, "command")?;
    ensure_not_file_write_command(&command)?;
    let timeout = command_timeout(&args);
    let output = run_shell_command(&command, timeout).await?;
    command_output(output)
}

/// 执行只读 shell 命令。
///
/// 参数:
/// - `args`: 工具参数
///
/// 返回:
/// - JSON 格式命令结果
async fn run_readonly_command(args: Value) -> Result<String> {
    let command = required(&args, "command")?;
    ensure_readonly_command(&command)?;
    let timeout = command_timeout(&args);
    let output = run_shell_command(&command, timeout).await?;
    command_output(output)
}

/// 读取并限制命令超时参数。
///
/// 参数:
/// - `args`: 工具参数
///
/// 返回:
/// - 超时秒数
fn command_timeout(args: &Value) -> u64 {
    args.get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(30)
        .clamp(1, 120)
}

/// 校验计划模式只读命令。
///
/// 参数:
/// - `command`: shell 命令文本
///
/// 返回:
/// - 命令是否允许
fn ensure_readonly_command(command: &str) -> Result<()> {
    let lower = command.to_ascii_lowercase();
    let forbidden = [
        ">",
        ">>",
        " 2>",
        "tee ",
        "tee -",
        "rm ",
        "mv ",
        "cp ",
        "mkdir ",
        "rmdir ",
        "touch ",
        "chmod ",
        "chown ",
        "chgrp ",
        "ln ",
        "truncate ",
        "dd ",
        "mkfs",
        "mount ",
        "umount ",
        "systemctl ",
        "service ",
        "kill ",
        "pkill ",
        "reboot",
        "shutdown",
        "poweroff",
        "pacman -s",
        "pacman -r",
        "pacman -u",
        "paru -s",
        "yay -s",
        "apt install",
        "apt remove",
        "apt update",
        "dnf install",
        "brew install",
        "sed -i",
        "git add",
        "git commit",
        "git push",
        "git reset",
        "git checkout",
        "cargo build",
        "cargo test",
        "make ",
        "npm install",
        "pnpm install",
        "yarn install",
        "remove-item",
        "ri ",
        " ri ",
        "del ",
        "erase ",
        "copy ",
        "move ",
        "ren ",
        "rename ",
        "move-item",
        "copy-item",
        "new-item",
        "rename-item",
        "set-content",
        "add-content",
        "clear-content",
        "out-file",
        "set-acl",
        "start-process",
        "stop-process",
        "taskkill",
        "winget install",
        "winget uninstall",
        "scoop install",
        "choco install",
    ];
    if forbidden.iter().any(|needle| lower.contains(needle)) {
        bail!(
            "{}",
            t(
                "Plan mode only allows read-only inspection commands",
                "计划模式只允许只读检查命令"
            )
        );
    }
    Ok(())
}

/// 禁止用 shell 命令代替专用文件写入工具。
///
/// 参数:
/// - `command`: shell 命令文本
///
/// 返回:
/// - 命令是否允许
fn ensure_not_file_write_command(command: &str) -> Result<()> {
    if has_output_redirection(command) || has_tee_write(command) || has_cat_heredoc(command) {
        bail!(
            "{}",
            t(
                "Use write_file for new/full-file content or edit_file for line edits instead of shell file writes",
                "请使用 write_file 新建或整文件写入，或使用 edit_file 做行级修改，不要用 shell 写文件"
            )
        );
    }
    Ok(())
}

/// 判断命令是否包含输出重定向写文件。
///
/// 参数:
/// - `command`: shell 命令文本
///
/// 返回:
/// - 是否包含 stdout 写文件重定向
fn has_output_redirection(command: &str) -> bool {
    let mut single = false;
    let mut double = false;
    let mut escaped = false;
    let chars = command.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if *ch == '\\' {
            escaped = true;
            continue;
        }
        if *ch == '\'' && !double {
            single = !single;
            continue;
        }
        if *ch == '"' && !single {
            double = !double;
            continue;
        }
        if *ch != '>' || single || double || previous_non_space(&chars, index) == Some('2') {
            continue;
        }
        if next_non_space(&chars, index) != Some('&') {
            return true;
        }
    }
    false
}

/// 判断命令是否通过 tee 写文件。
///
/// 参数:
/// - `command`: shell 命令文本
///
/// 返回:
/// - 是否包含 tee 写文件
fn has_tee_write(command: &str) -> bool {
    let words = shell_words(command);
    words
        .iter()
        .enumerate()
        .any(|(index, word)| is_tee_command(word) && has_tee_target(&words[index + 1..]))
}

/// 判断 shell 词是否是 tee 命令。
///
/// 参数:
/// - `word`: shell 词
///
/// 返回:
/// - 是否为 tee 命令
fn is_tee_command(word: &str) -> bool {
    word == "tee" || word.ends_with("/tee")
}

/// 判断 tee 后续参数是否包含写入目标。
///
/// 参数:
/// - `words`: tee 后面的 shell 词
///
/// 返回:
/// - 是否包含文件目标
fn has_tee_target(words: &[String]) -> bool {
    words
        .iter()
        .any(|word| word != "-" && !word.starts_with('-'))
}

/// 判断命令是否使用 cat heredoc。
///
/// 参数:
/// - `command`: shell 命令文本
///
/// 返回:
/// - 是否包含 cat heredoc
fn has_cat_heredoc(command: &str) -> bool {
    command.contains("<<")
        && shell_words(command)
            .iter()
            .any(|word| word == "cat" || word.ends_with("/cat"))
}

/// 返回指定位置前一个非空白字符。
///
/// 参数:
/// - `chars`: 字符列表
/// - `index`: 当前索引
///
/// 返回:
/// - 前一个非空白字符
fn previous_non_space(chars: &[char], index: usize) -> Option<char> {
    chars[..index]
        .iter()
        .rev()
        .copied()
        .find(|ch| !ch.is_whitespace())
}

/// 返回指定位置后一个非空白字符。
///
/// 参数:
/// - `chars`: 字符列表
/// - `index`: 当前索引
///
/// 返回:
/// - 后一个非空白字符
fn next_non_space(chars: &[char], index: usize) -> Option<char> {
    chars[index + 1..]
        .iter()
        .copied()
        .find(|ch| !ch.is_whitespace())
}

/// 提取未加引号的 shell 词。
///
/// 参数:
/// - `command`: shell 命令文本
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

/// 生成命令输出 JSON。
///
/// 参数:
/// - `output`: 进程输出
///
/// 返回:
/// - JSON 字符串
fn command_output(output: std::process::Output) -> Result<String> {
    let stdout = clip_output(&String::from_utf8_lossy(&output.stdout));
    let stderr = clip_output(&String::from_utf8_lossy(&output.stderr));
    Ok(serde_json::to_string_pretty(
        &json!({"success": output.status.success(), "exit_code": output.status.code(), "stdout": stdout, "stderr": stderr}),
    )?)
}

/// 截断命令输出。
///
/// 参数:
/// - `value`: 原始输出
///
/// 返回:
/// - 截断后的输出
fn clip_output(value: &str) -> String {
    let value = value.trim();
    if value.chars().count() <= MAX_COMMAND_OUTPUT_CHARS {
        value.to_string()
    } else {
        format!(
            "{}\n...[{} {MAX_COMMAND_OUTPUT_CHARS} {}]",
            value
                .chars()
                .take(MAX_COMMAND_OUTPUT_CHARS)
                .collect::<String>(),
            t("truncated to", "已截断到"),
            t("chars", "字符")
        )
    }
}

/// 读取必填字符串参数。
///
/// 参数:
/// - `args`: 工具参数
/// - `key`: 参数名
///
/// 返回:
/// - 参数值
fn required(args: &Value, key: &str) -> Result<String> {
    let value = args
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if value.is_empty() {
        bail!("{}: {key}", t("required argument missing", "缺少必需参数"))
    } else {
        Ok(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_command_allows_inspection() {
        assert!(ensure_readonly_command("git status --short").is_ok());
        assert!(ensure_readonly_command("pacman -Q miyu").is_ok());
    }

    #[test]
    fn readonly_command_blocks_mutation() {
        assert!(ensure_readonly_command("rm file").is_err());
        assert!(ensure_readonly_command("sed -i 's/a/b/' file").is_err());
        assert!(ensure_readonly_command("cargo test").is_err());
        assert!(ensure_readonly_command("Remove-Item file").is_err());
        assert!(ensure_readonly_command("Set-Content file value").is_err());
        assert!(ensure_readonly_command("winget install foo").is_err());
    }

    #[test]
    fn writable_command_rejects_shell_file_writes() {
        assert!(ensure_not_file_write_command("cat <<'EOF' > file\nx\nEOF").is_err());
        assert!(ensure_not_file_write_command("printf hello > file").is_err());
        assert!(ensure_not_file_write_command("echo hello >> file").is_err());
        assert!(ensure_not_file_write_command("printf hello | tee file").is_err());
        assert!(ensure_not_file_write_command("grep foo file 2>/dev/null").is_ok());
        assert!(ensure_not_file_write_command("tee --help").is_ok());
    }

    #[tokio::test]
    async fn readonly_command_runs_with_platform_shell() {
        #[cfg(windows)]
        let command = "Write-Output hello";
        #[cfg(not(windows))]
        let command = "printf hello";

        let result = run_readonly_command(json!({"command": command}))
            .await
            .unwrap();
        let data: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(data["success"], true);
        assert_eq!(data["stdout"], "hello");
    }
}
