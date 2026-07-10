use super::path_guard::{existing_path, writable_path};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::Path;
use std::time::UNIX_EPOCH;

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TREE_DEPTH: usize = 8;
const IGNORED_DIRECTORIES: &[&str] = &[".git", "node_modules", "target", "dist", "build"];

/// 文件树节点。
#[derive(Clone, Debug, Serialize)]
pub(crate) struct FileNode {
    pub name: String,
    pub path: String,
    pub kind: &'static str,
    pub children: Vec<FileNode>,
}

/// 文本文件内容。
#[derive(Clone, Debug, Serialize)]
pub(crate) struct FileContent {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub modified_at: Option<u64>,
}

/// 读取工作区文件树。
///
/// 参数:
/// - `root`: 工作区根目录
/// - `relative`: 起始相对目录
/// - `depth`: 最大递归深度
///
/// 返回:
/// - 文件树节点
pub(crate) fn read_tree(root: &Path, relative: &str, depth: usize) -> Result<Vec<FileNode>> {
    let path = existing_path(root, relative)?;
    if !path.is_dir() {
        bail!("tree path is not a directory");
    }
    read_directory(root, &path, depth.clamp(1, MAX_TREE_DEPTH))
}

/// 读取 UTF-8 文本文件。
pub(crate) fn read_file(root: &Path, relative: &str) -> Result<FileContent> {
    let path = existing_path(root, relative)?;
    let metadata = std::fs::metadata(&path)?;
    if !metadata.is_file() {
        bail!("path is not a file");
    }
    if metadata.len() > MAX_FILE_BYTES {
        bail!("file exceeds {} bytes", MAX_FILE_BYTES);
    }
    let bytes = std::fs::read(&path)?;
    if bytes.contains(&0) {
        bail!("binary files are not supported");
    }
    let content = String::from_utf8(bytes).context("file is not valid UTF-8")?;
    Ok(FileContent {
        path: relative.to_string(),
        content,
        size: metadata.len(),
        modified_at: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs()),
    })
}

/// 原子保存 UTF-8 文本文件。
pub(crate) fn write_file(root: &Path, relative: &str, content: &str) -> Result<FileContent> {
    if content.len() as u64 > MAX_FILE_BYTES {
        bail!("file exceeds {} bytes", MAX_FILE_BYTES);
    }
    let path = writable_path(root, relative)?;
    if path.exists() && !path.is_file() {
        bail!("path is not a regular file");
    }
    let parent = path.parent().context("file path has no parent")?;
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    std::fs::write(temp.path(), content.as_bytes())?;
    temp.persist(&path)?;
    read_file(root, relative)
}

/// 递归读取单个目录。
fn read_directory(root: &Path, directory: &Path, depth: usize) -> Result<Vec<FileNode>> {
    let mut entries = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| {
        let is_file = entry.file_type().map(|kind| kind.is_file()).unwrap_or(true);
        (
            is_file,
            entry.file_name().to_string_lossy().to_ascii_lowercase(),
        )
    });
    let mut nodes = Vec::new();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if IGNORED_DIRECTORIES.contains(&name.as_str()) {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type()?;
        let kind = if file_type.is_dir() {
            "directory"
        } else if file_type.is_symlink() {
            "symlink"
        } else {
            "file"
        };
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let children = if file_type.is_dir() && depth > 1 {
            read_directory(root, &path, depth - 1)?
        } else {
            Vec::new()
        };
        nodes.push(FileNode {
            name,
            path: relative,
            kind,
            children,
        });
    }
    Ok(nodes)
}
