use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use tokio::process::Command;

const OUTPUT_LIMIT: usize = 20_000;

/// REPL 本地 Shell 命令的执行结果。
pub(super) struct ReplShellResult {
    pub(super) command: String,
    pub(super) output: String,
    pub(super) exit_code: Option<i32>,
}

/// 执行以 `!` 开头的 REPL 本地 Shell 命令。
///
/// 参数:
/// - `command`: 不含 `!` 的 Shell 命令正文
///
/// 返回:
/// - 命令、合并后输出与退出码
pub(super) async fn execute_repl_shell(command: &str) -> Result<ReplShellResult> {
    let command = command.trim();
    if command.is_empty() {
        bail!("请在 ! 后输入 Shell 命令")
    }
    let shell = shell_path();
    let cwd = crate::runtime_cwd::current_dir()?;
    // 1. 使用当前用户 Shell 在 REPL 工作目录中执行命令
    let result = Command::new(&shell)
        .arg("-lc")
        .arg(command)
        .current_dir(cwd)
        .output()
        .await
        .with_context(|| format!("Shell 命令执行失败: {}", shell.display()))?;
    // 2. 按标准输出、标准错误顺序合并可见结果
    let mut output = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr);
    if !stderr.is_empty() {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(&stderr);
    }
    Ok(ReplShellResult {
        command: command.to_string(),
        output: truncate_output(&output),
        exit_code: result.status.code(),
    })
}

/// 返回当前环境的 Shell 路径。
///
/// 返回:
/// - `$SHELL` 或系统 zsh/sh
fn shell_path() -> PathBuf {
    std::env::var_os("SHELL")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .unwrap_or_else(|| {
            if PathBuf::from("/bin/zsh").exists() {
                PathBuf::from("/bin/zsh")
            } else {
                PathBuf::from("/bin/sh")
            }
        })
}

/// 限制 Shell 输出进入 transcript 的字符数。
///
/// 参数:
/// - `output`: 原始标准输出与标准错误
///
/// 返回:
/// - 限制后的输出
fn truncate_output(output: &str) -> String {
    if output.chars().count() <= OUTPUT_LIMIT {
        return output.to_string();
    }
    let mut truncated = output.chars().take(OUTPUT_LIMIT).collect::<String>();
    truncated.push_str("\n[Shell 输出已截断]");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn executes_shell_command_and_captures_output() {
        let result = execute_repl_shell("printf shell-test").await.unwrap();

        assert_eq!(result.command, "printf shell-test");
        assert_eq!(result.output, "shell-test");
        assert_eq!(result.exit_code, Some(0));
    }
}
