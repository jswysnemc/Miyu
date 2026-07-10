use anyhow::{bail, Context, Result};
use std::path::{Component, Path, PathBuf};

/// 解析并校验已经存在的工作区路径。
///
/// 参数:
/// - `root`: 工作区根目录
/// - `relative`: 工作区相对路径
///
/// 返回:
/// - 规范化后的绝对路径
pub(super) fn existing_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let root = root.canonicalize()?;
    let relative = validate_relative(relative)?;
    let path = root.join(relative);
    let canonical = path
        .canonicalize()
        .with_context(|| format!("path does not exist: {}", path.display()))?;
    if !canonical.starts_with(&root) {
        bail!("path escapes workspace root");
    }
    Ok(canonical)
}

/// 解析允许创建的新文件路径。
///
/// 参数:
/// - `root`: 工作区根目录
/// - `relative`: 工作区相对路径
///
/// 返回:
/// - 经过父目录校验的绝对路径
pub(super) fn writable_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let root = root.canonicalize()?;
    let relative = validate_relative(relative)?;
    let path = root.join(relative);
    let parent = path
        .parent()
        .context("file path has no parent directory")?
        .canonicalize()
        .with_context(|| format!("parent directory does not exist: {}", path.display()))?;
    if !parent.starts_with(&root) {
        bail!("path escapes workspace root");
    }
    Ok(path)
}

/// 解析允许重命名或删除的现有工作区条目。
///
/// 参数:
/// - `root`: 工作区根目录
/// - `relative`: 工作区相对路径
///
/// 返回:
/// - 不解析最终符号链接的绝对路径
pub(super) fn mutable_existing_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let root = root.canonicalize()?;
    let relative = validate_relative(relative)?;
    if relative == Path::new(".") {
        bail!("workspace root cannot be modified");
    }
    let path = root.join(relative);
    let parent = path
        .parent()
        .context("entry path has no parent directory")?
        .canonicalize()?;
    if !parent.starts_with(&root) {
        bail!("path escapes workspace root");
    }
    std::fs::symlink_metadata(&path)
        .with_context(|| format!("path does not exist: {}", path.display()))?;
    Ok(path)
}

/// 校验浏览器提交的路径为普通相对路径。
fn validate_relative(value: &str) -> Result<PathBuf> {
    let value = value.trim();
    let path = if value.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(value)
    };
    if path.is_absolute() {
        bail!("absolute paths are not allowed");
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        bail!("parent path components are not allowed");
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_and_absolute_paths() {
        assert!(validate_relative("../secret").is_err());
        assert!(validate_relative("/etc/passwd").is_err());
        assert!(validate_relative("src/main.rs").is_ok());
    }

    #[test]
    fn rejects_workspace_root_mutation() {
        let temp = tempfile::tempdir().unwrap();
        assert!(mutable_existing_path(temp.path(), "").is_err());
    }
}
