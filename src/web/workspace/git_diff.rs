use anyhow::Result;
use serde::Serialize;
use std::path::Path;
use tokio::process::Command;

/// 当前工作区 Git 状态与 Diff。
#[derive(Clone, Debug, Serialize)]
pub(crate) struct GitDiff {
    pub repository: bool,
    pub status: String,
    pub diff: String,
}

/// 读取 Git 工作区状态和未提交 Diff。
///
/// 参数:
/// - `root`: 工作区根目录
///
/// 返回:
/// - Git Diff 信息
pub(crate) async fn read_git_diff(root: &Path) -> Result<GitDiff> {
    let status = Command::new("git")
        .arg("status")
        .arg("--short")
        .current_dir(root)
        .output()
        .await?;
    if !status.status.success() {
        return Ok(GitDiff {
            repository: false,
            status: String::new(),
            diff: String::new(),
        });
    }
    let diff = Command::new("git")
        .arg("diff")
        .arg("--no-ext-diff")
        .arg("--")
        .current_dir(root)
        .output()
        .await?;
    Ok(GitDiff {
        repository: true,
        status: String::from_utf8_lossy(&status.stdout).into_owned(),
        diff: String::from_utf8_lossy(&diff.stdout).into_owned(),
    })
}
