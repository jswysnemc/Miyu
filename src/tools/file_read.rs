use super::{ToolRegistry, ToolSpec};
use crate::i18n::text as t;
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

const MAX_READ_BYTES: usize = 50 * 1024;
const MAX_BATCH_BYTES: usize = 100 * 1024;
const MAX_BATCH_FILES: usize = 10;
const MAX_READ_LINES: usize = 2_000;
const MAX_LINE_CHARS: usize = 2_000;

pub fn register(registry: &mut ToolRegistry) {
    registry.register(ToolSpec::new(
        "read_file",
        t(
            "Read one or more UTF-8 text files by 1-based line offset, or list directory pages. Use path for one target or files for batch reads. Large files are paged and binary files are refused.",
            "按 1 起始行号读取一个或多个 UTF-8 文本文件，或分页列出目录。单个目标使用 path，批量读取使用 files。大文件会分页，二进制文件会被拒绝。",
        ),
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": t("Single file or directory path.", "单个文件或目录路径。")
                },
                "offset": {
                    "type": "integer",
                    "description": t("Starting line or directory entry, 1-based. Defaults to 1.", "起始行或目录项，1 起始。默认 1。")
                },
                "limit": {
                    "type": "integer",
                    "description": t("Maximum lines or directory entries. Defaults to 2000.", "最多读取行数或目录项数量。默认 2000。")
                },
                "files": {
                    "type": "array",
                    "description": t("Batch file or directory pages. Each item supports path, offset, and limit.", "批量文件或目录分页。每一项支持 path、offset、limit。"),
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": t("File or directory path.", "文件或目录路径。")},
                            "offset": {"type": "integer", "description": t("Starting line or directory entry, 1-based.", "起始行或目录项，1 起始。")},
                            "limit": {"type": "integer", "description": t("Maximum lines or directory entries.", "最多读取行数或目录项数量。")}
                        },
                        "required": ["path"],
                        "additionalProperties": false
                    },
                    "maxItems": MAX_BATCH_FILES
                }
            },
            "additionalProperties": false
        }),
        |args| async move { read_file(args) },
    ));
}

/// 读取单个或批量文件内容。
///
/// 参数:
/// - `args`: 工具参数，支持 path 或 files
///
/// 返回:
/// - JSON 格式读取结果
fn read_file(args: Value) -> Result<String> {
    if let Some(files) = args.get("files") {
        return read_files(files);
    }
    let request = ReadRequest::from_value(&args)?;
    Ok(serde_json::to_string_pretty(&read_page(
        &request,
        MAX_READ_BYTES,
    )?)?)
}

/// 批量读取多个文件或目录分页。
///
/// 参数:
/// - `files`: 批量读取项
///
/// 返回:
/// - JSON 格式批量读取结果
fn read_files(files: &Value) -> Result<String> {
    let Some(items) = files.as_array() else {
        bail!("files must be an array")
    };
    if items.is_empty() {
        bail!("files must not be empty")
    }
    if items.len() > MAX_BATCH_FILES {
        bail!("files contains too many items: max {MAX_BATCH_FILES}")
    }
    let mut used_bytes = 0usize;
    let mut results = Vec::new();
    for item in items {
        if used_bytes >= MAX_BATCH_BYTES {
            results.push(json!({
                "ok": false,
                "type": "error",
                "path": item.get("path").and_then(Value::as_str).unwrap_or_default(),
                "error": "batch output byte limit reached before reading this item",
            }));
            continue;
        }
        let remaining = MAX_BATCH_BYTES
            .saturating_sub(used_bytes)
            .min(MAX_READ_BYTES);
        let result = match ReadRequest::from_value(item)
            .and_then(|request| read_page(&request, remaining))
        {
            Ok(value) => {
                used_bytes += value.to_string().len();
                value
            }
            Err(err) => json!({
                "ok": false,
                "type": "error",
                "path": item.get("path").and_then(Value::as_str).unwrap_or_default(),
                "error": err.to_string(),
            }),
        };
        results.push(result);
    }
    Ok(serde_json::to_string_pretty(&json!({
        "type": "multi-text-page",
        "count": results.len(),
        "max_batch_bytes": MAX_BATCH_BYTES,
        "results": results,
    }))?)
}

struct ReadRequest {
    path: PathBuf,
    offset: usize,
    limit: usize,
}

impl ReadRequest {
    /// 从 JSON 参数解析读取请求。
    ///
    /// 参数:
    /// - `args`: 单个读取请求参数
    ///
    /// 返回:
    /// - 读取请求
    fn from_value(args: &Value) -> Result<Self> {
        Ok(Self {
            path: path_arg(args, "path")?,
            offset: args
                .get("offset")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .max(1) as usize,
            limit: args
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(MAX_READ_LINES as u64)
                .clamp(1, MAX_READ_LINES as u64) as usize,
        })
    }
}

/// 读取一个路径的分页内容。
///
/// 参数:
/// - `request`: 读取请求
/// - `byte_budget`: 本次读取最大字节预算
///
/// 返回:
/// - JSON 值形式的分页内容
fn read_page(request: &ReadRequest, byte_budget: usize) -> Result<Value> {
    if request.path.is_dir() {
        return read_directory_page(request);
    }
    let metadata = std::fs::metadata(&request.path)?;
    if !metadata.is_file() {
        bail!(
            "not a regular file or directory: {}",
            request.path.display()
        )
    }
    ensure_not_binary_file(&request.path)?;
    read_text_page(request, byte_budget)
}

/// 读取目录分页。
///
/// 参数:
/// - `request`: 读取请求
///
/// 返回:
/// - 目录分页 JSON
fn read_directory_page(request: &ReadRequest) -> Result<Value> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&request.path)? {
        let entry = entry?;
        let suffix = if entry.file_type()?.is_dir() { "/" } else { "" };
        entries.push(format!("{}{}", entry.file_name().to_string_lossy(), suffix));
    }
    entries.sort();
    let start = request.offset.saturating_sub(1);
    let selected = entries
        .iter()
        .skip(start)
        .take(request.limit)
        .cloned()
        .collect::<Vec<_>>();
    let next = (start + selected.len() < entries.len()).then_some(request.offset + selected.len());
    Ok(json!({
        "type": "directory-page",
        "path": request.path.display().to_string(),
        "offset": request.offset,
        "limit": request.limit,
        "entries": selected,
        "truncated": next.is_some(),
        "next": next,
    }))
}

/// 读取文本文件分页。
///
/// 参数:
/// - `request`: 读取请求
/// - `byte_budget`: 本次读取最大字节预算
///
/// 返回:
/// - 文本分页 JSON
fn read_text_page(request: &ReadRequest, byte_budget: usize) -> Result<Value> {
    let file = std::fs::File::open(&request.path)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    let mut bytes = 0usize;
    let mut next = None;
    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        if line_number < request.offset {
            continue;
        }
        if lines.len() >= request.limit || bytes >= byte_budget {
            next = Some(line_number);
            break;
        }
        let mut line = line?;
        if line.chars().count() > MAX_LINE_CHARS {
            line = format!(
                "{}... (line truncated to {MAX_LINE_CHARS} chars)",
                line.chars().take(MAX_LINE_CHARS).collect::<String>()
            );
        }
        let rendered = format!("{line_number}: {line}");
        bytes += rendered.len() + 1;
        if bytes > byte_budget {
            next = Some(line_number);
            break;
        }
        lines.push(rendered);
    }
    if lines.is_empty() && request.offset != 1 {
        bail!("offset {} is out of range", request.offset)
    }
    Ok(json!({
        "type": "text-page",
        "path": request.path.display().to_string(),
        "offset": request.offset,
        "limit": request.limit,
        "content": lines.join("\n"),
        "truncated": next.is_some(),
        "next": next,
    }))
}

/// 检查文件是否看起来是二进制文件。
///
/// 参数:
/// - `path`: 文件路径
///
/// 返回:
/// - 文件是否可作为文本读取
fn ensure_not_binary_file(path: &Path) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0u8; 8192];
    let read = file.read(&mut buffer)?;
    let sample = &buffer[..read];
    if sample.contains(&0) {
        bail!("cannot read binary file: {}", path.display())
    }
    let non_printable = sample
        .iter()
        .filter(|byte| **byte < 9 || (**byte > 13 && **byte < 32))
        .count();
    if !sample.is_empty() && non_printable * 10 > sample.len() * 3 {
        bail!("cannot read binary file: {}", path.display())
    }
    Ok(())
}

/// 读取必填路径参数。
///
/// 参数:
/// - `args`: JSON 参数
/// - `key`: 字段名
///
/// 返回:
/// - 展开后的路径
fn path_arg(args: &Value, key: &str) -> Result<PathBuf> {
    let value = args
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if value.is_empty() {
        bail!("{}: {key}", t("required argument missing", "缺少必需参数"))
    }
    Ok(expand_path(value))
}

/// 展开路径文本。
///
/// 参数:
/// - `value`: 路径文本
///
/// 返回:
/// - 绝对或当前目录相对路径
fn expand_path(value: &str) -> PathBuf {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix("~/") {
        if let Some(home) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
            return home.join(rest);
        }
    }
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_file_paginates_text() {
        let cwd = std::env::current_dir().unwrap();
        let temp = tempfile::tempdir_in(cwd).unwrap();
        let path = temp.path().join("sample.txt");
        std::fs::write(&path, "one\ntwo\nthree\n").unwrap();
        let result = read_file(json!({
            "path": path.display().to_string(),
            "offset": 2,
            "limit": 1,
        }))
        .unwrap();
        let data: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(data["type"], "text-page");
        assert_eq!(data["content"], "2: two");
        assert_eq!(data["truncated"], true);
        assert_eq!(data["next"], 3);
    }

    #[test]
    fn read_file_reads_multiple_files() {
        let cwd = std::env::current_dir().unwrap();
        let temp = tempfile::tempdir_in(cwd).unwrap();
        let first = temp.path().join("first.txt");
        let second = temp.path().join("second.txt");
        std::fs::write(&first, "a1\na2\n").unwrap();
        std::fs::write(&second, "b1\nb2\n").unwrap();
        let result = read_file(json!({
            "files": [
                {"path": first.display().to_string(), "offset": 2, "limit": 1},
                {"path": second.display().to_string(), "limit": 1}
            ]
        }))
        .unwrap();
        let data: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(data["type"], "multi-text-page");
        assert_eq!(data["count"], 2);
        assert_eq!(data["results"][0]["content"], "2: a2");
        assert_eq!(data["results"][1]["content"], "1: b1");
        assert_eq!(data["results"][1]["next"], 2);
    }

    #[test]
    fn read_file_batch_keeps_item_errors_local() {
        let cwd = std::env::current_dir().unwrap();
        let temp = tempfile::tempdir_in(cwd).unwrap();
        let text = temp.path().join("sample.txt");
        let bin = temp.path().join("sample.bin");
        std::fs::write(&text, "ok\n").unwrap();
        std::fs::write(&bin, [0, 1, 2, 3]).unwrap();
        let result = read_file(json!({
            "files": [
                {"path": text.display().to_string()},
                {"path": bin.display().to_string()}
            ]
        }))
        .unwrap();
        let data: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(data["results"][0]["type"], "text-page");
        assert_eq!(data["results"][1]["ok"], false);
        assert!(data["results"][1]["error"]
            .as_str()
            .unwrap()
            .contains("cannot read binary file"));
    }

    #[test]
    fn read_file_rejects_binary() {
        let cwd = std::env::current_dir().unwrap();
        let temp = tempfile::tempdir_in(cwd).unwrap();
        let path = temp.path().join("sample.bin");
        std::fs::write(&path, [0, 1, 2, 3]).unwrap();
        assert!(read_file(json!({"path": path.display().to_string()})).is_err());
    }
}
