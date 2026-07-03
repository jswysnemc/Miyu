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
        t("Run a shell command in the workspace when skills.allow_command_execution is enabled.", "当 skills.allow_command_execution 启用时，在工作区运行 shell 命令。"),
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
