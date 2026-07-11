use anyhow::{bail, Result};
use serde::Serialize;
use std::path::Path;
use tokio::process::Command;

/// 单个 Git 文件状态。
#[derive(Clone, Debug, Serialize)]
pub(crate) struct GitFileStatus {
    pub path: String,
    pub index_status: String,
    pub worktree_status: String,
}

/// 当前工作区 Git 状态与 Diff。
#[derive(Clone, Debug, Serialize)]
pub(crate) struct GitDiff {
    pub repository: bool,
    pub branch: String,
    pub status: String,
    pub files: Vec<GitFileStatus>,
    pub diff: String,
}

/// 读取 Git 工作区状态、分支和未提交 Diff。
///
/// 参数:
/// - `root`: 工作区根目录
///
/// 返回:
/// - Git 状态信息
pub(crate) async fn read_git_diff(root: &Path) -> Result<GitDiff> {
    let status = Command::new("git")
        .args(["status", "--porcelain=v1", "--branch"])
        .current_dir(root)
        .output()
        .await?;
    if !status.status.success() {
        return Ok(empty_git_diff());
    }
    let status_text = String::from_utf8_lossy(&status.stdout).into_owned();
    let (branch, files) = parse_status(&status_text);
    let unstaged = git_output(root, &["diff", "--no-ext-diff", "--"]).await?;
    let staged = git_output(root, &["diff", "--cached", "--no-ext-diff", "--"]).await?;
    let diff = match (staged.trim().is_empty(), unstaged.trim().is_empty()) {
        (true, _) => unstaged,
        (_, true) => staged,
        (false, false) => format!("# Staged changes\n{staged}\n# Working tree changes\n{unstaged}"),
    };
    Ok(GitDiff {
        repository: true,
        branch,
        status: status_text,
        files,
        diff,
    })
}

/// 执行经过约束的 Git 工作区操作。
///
/// 参数:
/// - `root`: 工作区根目录
/// - `action`: stage、unstage、discard 或 commit
/// - `paths`: 操作涉及的相对路径，空列表表示全部
/// - `message`: 提交说明
///
/// 返回:
/// - 操作后的 Git 状态
pub(crate) async fn apply_git_action(
    root: &Path,
    action: &str,
    paths: &[String],
    message: Option<&str>,
) -> Result<GitDiff> {
    match action {
        "init" => {
            let branch = message
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("main");
            run_git(root, &["init", "-b", branch]).await?;
        }
        "stage" => run_paths_command(root, &["add"], paths, "-A").await?,
        "unstage" => run_paths_command(root, &["reset", "-q", "HEAD"], paths, ".").await?,
        "discard" => {
            if paths.is_empty() {
                bail!("discard requires at least one path");
            }
            run_paths_command(root, &["restore", "--worktree"], paths, "").await?;
        }
        "commit" => {
            let message = message
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow::anyhow!("commit message cannot be empty"))?;
            run_git(root, &["commit", "-m", message]).await?;
        }
        _ => bail!("unsupported git action: {action}"),
    }
    read_git_diff(root).await
}

/// 执行带路径列表的 Git 命令。
async fn run_paths_command(
    root: &Path,
    prefix: &[&str],
    paths: &[String],
    all_path: &str,
) -> Result<()> {
    let mut command = Command::new("git");
    command.args(prefix).current_dir(root);
    if paths.is_empty() && !all_path.is_empty() {
        command.arg(all_path);
    } else {
        command.arg("--").args(paths);
    }
    let output = command.output().await?;
    ensure_success(output)
}

/// 执行固定参数 Git 命令。
async fn run_git(root: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .await?;
    ensure_success(output)
}

/// 读取固定参数 Git 命令标准输出。
async fn git_output(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .await?;
    if !output.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 校验 Git 命令退出状态。
fn ensure_success(output: std::process::Output) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if message.is_empty() {
        bail!("git command failed");
    }
    bail!("{message}")
}

/// 解析 porcelain 状态文本。
fn parse_status(status: &str) -> (String, Vec<GitFileStatus>) {
    let mut branch = String::new();
    let mut files = Vec::new();
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("## ") {
            branch = value
                .strip_prefix("No commits yet on ")
                .unwrap_or(value)
                .split("...")
                .next()
                .unwrap_or(value)
                .to_string();
            continue;
        }
        if line.len() < 4 {
            continue;
        }
        let path = line[3..]
            .split(" -> ")
            .last()
            .unwrap_or(&line[3..])
            .to_string();
        files.push(GitFileStatus {
            path,
            index_status: line[0..1].to_string(),
            worktree_status: line[1..2].to_string(),
        });
    }
    (branch, files)
}

/// 返回非 Git 仓库使用的空状态。
fn empty_git_diff() -> GitDiff {
    GitDiff {
        repository: false,
        branch: String::new(),
        status: String::new(),
        files: Vec::new(),
        diff: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_and_file_status() {
        let (branch, files) = parse_status("## main...origin/main\nM  src/main.rs\n?? notes.md\n");
        assert_eq!(branch, "main");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].index_status, "M");
        assert_eq!(files[1].worktree_status, "?");
    }
}
