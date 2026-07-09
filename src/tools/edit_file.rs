use super::edit_patch::{apply_patch, FileChange};
use super::{ToolRegistry, ToolSpec};
use crate::i18n::text as t;
use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// 注册编辑文件工具。
///
/// 参数:
/// - `registry`: 工具注册表
///
/// 返回:
/// - 无
pub(crate) fn register(registry: &mut ToolRegistry) {
    registry.register(
        ToolSpec::new(
            "edit_file",
            t(
                "Create, overwrite, or edit UTF-8 text files. Prefer the patch parameter with Codex-style syntax: {\"patch\":\"*** Begin Patch\\n*** Update File: src/main.rs\\n@@\\n-old\\n+new\\n*** End Patch\"}. Use *** Add File and *** Delete File for creates/deletes. Include enough unchanged context lines so the hunk matches exactly. Legacy modes remain supported: path+content for full-file writes, or path+start_line+end_line+replacement after read_file identifies exact line numbers.",
                "创建、覆盖或编辑 UTF-8 文本文件。优先使用 patch 参数和 Codex 风格语法，例如 {\"patch\":\"*** Begin Patch\\n*** Update File: src/main.rs\\n@@\\n-old\\n+new\\n*** End Patch\"}。新建和删除分别使用 *** Add File 与 *** Delete File。hunk 中要包含足够的未变上下文行，保证精确匹配。仍兼容 path+content 整文件写入，以及 read_file 确认行号后的 path+start_line+end_line+replacement 行级编辑。",
            ),
            json!({"type":"object","properties":{"patch":{"type":"string","description": t("Preferred edit mode. Codex-style patch beginning with *** Begin Patch. Example: *** Begin Patch\\n*** Update File: src/main.rs\\n@@\\n-old\\n+new\\n*** End Patch. Also supports *** Add File and *** Delete File.", "推荐编辑模式。以 *** Begin Patch 开头的 Codex 风格 patch。例如 *** Begin Patch\\n*** Update File: src/main.rs\\n@@\\n-old\\n+new\\n*** End Patch。也支持 *** Add File 和 *** Delete File。")},"path":{"type":"string","description": t("Workspace-relative or absolute file path for legacy edit modes.", "旧编辑模式使用的工作区相对路径或绝对文件路径。")},"content":{"type":"string","description": t("Legacy full UTF-8 file content for create or overwrite. Prefer patch for multi-line edits.", "旧整文件写入模式，用于新建或覆盖完整 UTF-8 文件。多行编辑优先使用 patch。")},"start_line":{"type":"integer","description": t("Legacy 1-based first line to replace.", "旧行级编辑模式：要替换的第一行，1 起始。")},"end_line":{"type":"integer","description": t("Legacy 1-based last line to replace, inclusive.", "旧行级编辑模式：要替换的最后一行，闭区间。")},"replacement":{"type":"string","description": t("Legacy replacement text. May contain multiple lines. Empty text deletes the line range. Prefer patch when possible.", "旧行级编辑模式：替换文本，可包含多行；空文本会删除指定行范围。可行时优先使用 patch。")}},"additionalProperties":false}),
            |args| async move { edit_file(args) },
        )
        .writes(),
    );
}

/// 创建、覆盖或编辑 UTF-8 文本文件。
///
/// 参数:
/// - `args`: 工具参数，优先包含 patch；旧模式包含 content 或行号替换参数
///
/// 返回:
/// - JSON 格式编辑结果
fn edit_file(args: Value) -> Result<String> {
    if let Some(patch) = args.get("patch").and_then(Value::as_str) {
        let cwd = std::env::current_dir()?;
        let applied = apply_patch(patch, &cwd)?;
        return Ok(serde_json::to_string_pretty(&json!({
            "ok": true,
            "mode": "patch",
            "changed_files": applied.changes.iter().map(file_change_summary).collect::<Vec<_>>()
        }))?);
    }
    if args.get("content").is_some() {
        return write_file(args);
    }
    edit_file_lines(args)
}

/// 创建或覆盖 UTF-8 文本文件。
///
/// 参数:
/// - `args`: 工具参数，包含 path 和 content
///
/// 返回:
/// - JSON 格式写入结果
fn write_file(args: Value) -> Result<String> {
    let path = path_arg(&args, "path")?;
    let content = args
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("content is required"))?;
    if path.exists() && !path.is_file() {
        bail!("not a regular file: {}", path.display())
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let old_bytes = std::fs::metadata(&path).ok().map(|metadata| metadata.len());
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    std::fs::write(temp.path(), content.as_bytes())?;
    temp.persist(&path)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "mode": "content",
        "path": path.display().to_string(),
        "old_bytes": old_bytes,
        "new_bytes": content.len()
    }))?)
}

/// 按行号替换 UTF-8 文本文件。
///
/// 参数:
/// - `args`: 工具参数，包含 path、start_line、end_line 和 replacement
///
/// 返回:
/// - JSON 格式编辑结果
fn edit_file_lines(args: Value) -> Result<String> {
    let path = path_arg(&args, "path")?;
    ensure_editable_file_path(&path)?;
    let start_line = args
        .get("start_line")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("start_line is required"))? as usize;
    let end_line = args
        .get("end_line")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow::anyhow!("end_line is required"))? as usize;
    let replacement = args
        .get("replacement")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("replacement is required"))?;
    if start_line == 0 || end_line == 0 {
        bail!("line numbers must be 1-based")
    }
    if start_line > end_line {
        bail!("start_line must be less than or equal to end_line")
    }
    let original = std::fs::read_to_string(&path)?;
    let had_trailing_newline = original.ends_with('\n');
    let mut lines = original.lines().map(str::to_string).collect::<Vec<_>>();
    let old_line_count = lines.len();
    if start_line > old_line_count || end_line > old_line_count {
        bail!("line range {start_line}-{end_line} out of range: {old_line_count} lines")
    }
    let replacement = replacement.replace("\r\n", "\n").replace('\r', "\n");
    let replacement_lines = if replacement.is_empty() {
        Vec::new()
    } else {
        replacement.lines().map(str::to_string).collect::<Vec<_>>()
    };
    lines.splice(start_line - 1..end_line, replacement_lines);
    let mut updated = lines.join("\n");
    if had_trailing_newline && !updated.is_empty() {
        updated.push('\n');
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    std::fs::write(temp.path(), updated.as_bytes())?;
    temp.persist(&path)?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "mode": "line",
        "path": path.display().to_string(),
        "old_line_count": old_line_count,
        "new_line_count": lines.len()
    }))?)
}

/// 生成文件变更摘要。
///
/// 参数:
/// - `change`: 文件变更
///
/// 返回:
/// - JSON 文件变更摘要
fn file_change_summary(change: &FileChange) -> Value {
    let (added, removed) = change.line_counts();
    let mut value = json!({
        "action": change.action_label(),
        "path": change.path().display().to_string(),
        "added": added,
        "removed": removed
    });
    if let FileChange::Update {
        move_path: Some(move_path),
        ..
    } = change
    {
        value["move_path"] = json!(move_path.display().to_string());
    }
    value
}

/// 确认路径是可编辑普通文件。
///
/// 参数:
/// - `path`: 文件路径
///
/// 返回:
/// - 校验是否成功
fn ensure_editable_file_path(path: &Path) -> Result<()> {
    let canonical = path.canonicalize()?;
    if !canonical.is_file() {
        bail!("not a regular file: {}", path.display())
    }
    Ok(())
}

/// 读取路径参数。
///
/// 参数:
/// - `args`: 工具参数
/// - `key`: 参数名
///
/// 返回:
/// - 展开后的路径
fn path_arg(args: &Value, key: &str) -> Result<PathBuf> {
    let value = required(args, key)?;
    Ok(expand_path(&value))
}

/// 展开工具路径。
///
/// 参数:
/// - `value`: 原始路径文本
///
/// 返回:
/// - 绝对路径或当前工作目录相对路径
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

/// 读取必填字符串参数。
///
/// 参数:
/// - `args`: 工具参数
/// - `key`: 参数名
///
/// 返回:
/// - 非空参数值
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
    fn write_file_creates_and_overwrites_text_file() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("nested").join("sample.txt");
        write_file(json!({"path": path.display().to_string(), "content": "one\n"})).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "one\n");
        let result =
            write_file(json!({"path": path.display().to_string(), "content": "two"})).unwrap();
        let data: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(data["old_bytes"], 4);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "two");
    }

    #[test]
    fn edit_file_replaces_lines() {
        let cwd = std::env::current_dir().unwrap();
        let temp = tempfile::tempdir_in(cwd).unwrap();
        let path = temp.path().join("sample.txt");
        std::fs::write(&path, "one\ntwo\nthree\n").unwrap();
        edit_file(json!({
            "path": path.display().to_string(),
            "start_line": 2,
            "end_line": 2,
            "replacement": "TWO\nTWO-B"
        }))
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            "one\nTWO\nTWO-B\nthree\n"
        );
    }

    #[test]
    fn edit_file_applies_patch() {
        let cwd = std::env::current_dir().unwrap();
        let temp = tempfile::tempdir_in(cwd).unwrap();
        let path = temp.path().join("sample.txt");
        std::fs::write(&path, "one\ntwo\n").unwrap();
        let patch = format!(
            "*** Begin Patch\n*** Update File: {}\n@@\n-one\n+ONE\n two\n*** End Patch",
            path.display()
        );

        let result = edit_file(json!({ "patch": patch })).unwrap();
        let data: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(data["changed_files"][0]["action"], "Edited");
        assert_eq!(std::fs::read_to_string(path).unwrap(), "ONE\ntwo\n");
    }

    #[test]
    fn edit_file_allows_existing_files_outside_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("sample.txt");
        std::fs::write(&path, "one\ntwo\n").unwrap();
        edit_file(json!({
            "path": path.display().to_string(),
            "start_line": 1,
            "end_line": 2,
            "replacement": "table"
        }))
        .unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "table\n");
    }
}
