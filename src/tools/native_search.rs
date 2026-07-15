use anyhow::{Context, Result};
use regex::{Regex, RegexBuilder};
use std::path::Path;

/// 内置搜索结果。
pub(crate) struct NativeSearchResult {
    pub(crate) lines: Vec<String>,
    pub(crate) truncated: bool,
}

/// 递归查找匹配文件名或相对路径的文件。
///
/// 参数:
/// - `root`: 搜索根目录
/// - `pattern`: 大小写不敏感 Glob 模式
/// - `max_results`: 最大结果数
///
/// 返回:
/// - 匹配文件列表
pub(crate) fn glob_files(
    root: &Path,
    pattern: &str,
    max_results: usize,
) -> Result<NativeSearchResult> {
    let matcher = glob_matcher(pattern)?;
    let mut lines = Vec::new();
    // 1. 遍历普通文件并匹配统一使用正斜杠的相对路径
    visit_files(root, &mut |path| {
        let relative = relative_display(root, path);
        if matcher.is_match(&relative) {
            lines.push(relative);
        }
        lines.len() > max_results
    })?;
    // 2. 保留最大结果数并记录是否发生截断
    Ok(limit_results(lines, max_results))
}

/// 递归搜索 UTF-8 文本文件内容。
///
/// 参数:
/// - `root`: 搜索根目录
/// - `single_file`: 仅搜索指定文件
/// - `pattern`: 正则表达式
/// - `include`: 可选文件 Glob 过滤
/// - `max_results`: 最大结果数
///
/// 返回:
/// - 带文件名和行号的匹配结果
pub(crate) fn grep_text(
    root: &Path,
    single_file: Option<&Path>,
    pattern: &str,
    include: Option<&str>,
    max_results: usize,
) -> Result<NativeSearchResult> {
    let matcher =
        Regex::new(pattern).with_context(|| format!("invalid regex pattern: {pattern}"))?;
    let include = include.map(glob_matcher).transpose()?;
    let mut lines = Vec::new();
    // 1. 读取指定文件或递归遍历目录中的普通文件
    if let Some(path) = single_file {
        search_file(
            root,
            path,
            &matcher,
            include.as_ref(),
            max_results,
            &mut lines,
        )?;
    } else {
        visit_files(root, &mut |path| {
            let _ = search_file(
                root,
                path,
                &matcher,
                include.as_ref(),
                max_results,
                &mut lines,
            );
            lines.len() > max_results
        })?;
    }
    // 2. 保留最大结果数并记录是否发生截断
    Ok(limit_results(lines, max_results))
}

/// 搜索单个文本文件。
///
/// 参数:
/// - `root`: 搜索根目录
/// - `path`: 文件路径
/// - `matcher`: 内容正则表达式
/// - `include`: 可选文件 Glob 过滤器
/// - `max_results`: 最大结果数
/// - `lines`: 匹配结果集合
///
/// 返回:
/// - 搜索是否成功
fn search_file(
    root: &Path,
    path: &Path,
    matcher: &Regex,
    include: Option<&Regex>,
    max_results: usize,
    lines: &mut Vec<String>,
) -> Result<()> {
    let relative = relative_display(root, path);
    if include.is_some_and(|include| !include.is_match(&relative)) {
        return Ok(());
    }
    let bytes = std::fs::read(path)?;
    if bytes.contains(&0) {
        return Ok(());
    }
    let content = String::from_utf8_lossy(&bytes);
    for (index, line) in content.lines().enumerate() {
        if matcher.is_match(line) {
            lines.push(format!("{relative}:{}:{line}", index + 1));
            if lines.len() > max_results {
                break;
            }
        }
    }
    Ok(())
}

/// 递归访问目录中的普通文件，跳过 Git 元数据目录。
///
/// 参数:
/// - `root`: 搜索根目录
/// - `visitor`: 文件访问回调；返回真时停止遍历
///
/// 返回:
/// - 遍历是否成功
fn visit_files(root: &Path, visitor: &mut impl FnMut(&Path) -> bool) -> Result<()> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let mut entries = std::fs::read_dir(&directory)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                if entry.file_name() != ".git" {
                    pending.push(path);
                }
            } else if file_type.is_file() && visitor(&path) {
                return Ok(());
            }
        }
    }
    Ok(())
}

/// 将 Glob 模式转换为大小写不敏感正则表达式。
///
/// 参数:
/// - `pattern`: Glob 模式
///
/// 返回:
/// - 正则表达式匹配器
fn glob_matcher(pattern: &str) -> Result<Regex> {
    let mut regex = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '/' | '\\' => regex.push('/'),
            other => regex.push_str(&regex::escape(&other.to_string())),
        }
    }
    regex.push('$');
    RegexBuilder::new(&regex)
        .case_insensitive(true)
        .build()
        .with_context(|| format!("invalid glob pattern: {pattern}"))
}

/// 返回相对搜索根目录的跨平台显示路径。
///
/// 参数:
/// - `root`: 搜索根目录
/// - `path`: 文件绝对路径
///
/// 返回:
/// - 使用正斜杠分隔的相对路径
fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// 限制搜索结果数量。
///
/// 参数:
/// - `lines`: 全部匹配结果
/// - `max_results`: 最大结果数
///
/// 返回:
/// - 截断后的搜索结果
fn limit_results(mut lines: Vec<String>, max_results: usize) -> NativeSearchResult {
    let truncated = lines.len() > max_results;
    lines.truncate(max_results);
    NativeSearchResult { lines, truncated }
}
