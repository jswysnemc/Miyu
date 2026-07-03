use crate::render::asset_block;
use crate::render::code_block::{highlight_code_line, render_code_footer, render_code_header};
use crate::render::markdown_blocks;
#[cfg(test)]
pub(crate) use crate::render::markdown_inline::render_table_cell;
pub(crate) use crate::render::markdown_inline::{render_inline, render_table_cell_content};
use crate::render::style::{HEADER_STYLE, RESET, TERTIARY_STYLE};
use crate::render::table;

pub(crate) struct MarkdownStreamRenderer {
    buffer: String,
    line_renderer: MarkdownLineRenderer,
}

impl MarkdownStreamRenderer {
    /// 创建流式 Markdown 渲染器。
    ///
    /// 返回:
    /// - 新的流式渲染器
    pub(crate) fn new() -> Self {
        Self {
            buffer: String::new(),
            line_renderer: MarkdownLineRenderer::new(),
        }
    }

    /// 推入流式 Markdown 增量。
    ///
    /// 参数:
    /// - `delta`: 新收到的 Markdown 文本片段
    ///
    /// 返回:
    /// - 已完整渲染的终端文本
    pub(crate) fn push(&mut self, delta: &str) -> String {
        self.buffer.push_str(delta);
        let mut output = String::new();
        while let Some(index) = self.buffer.find('\n') {
            let line = self.buffer[..index].to_string();
            self.buffer = self.buffer[index + 1..].to_string();
            output.push_str(&self.line_renderer.render_line(&line));
        }
        output
    }

    /// 刷新剩余 Markdown 缓冲。
    ///
    /// 返回:
    /// - 最后一段渲染文本
    pub(crate) fn flush(&mut self) -> String {
        let mut output = String::new();
        if !self.buffer.is_empty() {
            let line = std::mem::take(&mut self.buffer);
            output.push_str(&self.line_renderer.render_line(&line));
        }
        output.push_str(&self.line_renderer.flush());
        output
    }
}

struct MarkdownLineRenderer {
    in_code_block: bool,
    in_math_block: bool,
    code_lang: String,
    code_buffer: Vec<String>,
    code_is_asset: bool,
    just_closed_code_block: bool,
    math_buffer: Vec<String>,
    table_buffer: Vec<String>,
}

impl MarkdownLineRenderer {
    /// 创建按行 Markdown 渲染器。
    ///
    /// 返回:
    /// - 新的按行渲染器
    fn new() -> Self {
        Self {
            in_code_block: false,
            in_math_block: false,
            code_lang: String::new(),
            code_buffer: Vec::new(),
            code_is_asset: false,
            just_closed_code_block: false,
            math_buffer: Vec::new(),
            table_buffer: Vec::new(),
        }
    }

    /// 渲染单行 Markdown。
    ///
    /// 参数:
    /// - `line`: 单行 Markdown 文本
    ///
    /// 返回:
    /// - 当前可输出的终端文本
    fn render_line(&mut self, line: &str) -> String {
        let skip_empty = std::mem::take(&mut self.just_closed_code_block);
        if skip_empty && !self.in_code_block && line.trim().is_empty() {
            return String::new();
        }

        if line.trim_start().starts_with("```") {
            if self.in_code_block {
                self.in_code_block = false;
                let lang = std::mem::take(&mut self.code_lang);
                let lines = std::mem::take(&mut self.code_buffer);
                if self.code_is_asset {
                    asset_block::render_asset_block(&lang, &lines)
                } else {
                    self.just_closed_code_block = true;
                    render_code_footer(&lines)
                }
            } else {
                let pending = self.flush();
                self.in_code_block = true;
                self.code_lang = line
                    .trim_start()
                    .trim_start_matches('`')
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                self.code_is_asset = asset_block::is_asset_language(&self.code_lang);
                self.code_buffer.clear();
                if self.code_is_asset {
                    pending
                } else {
                    pending + &render_code_header(&self.code_lang)
                }
            }
        } else if self.in_code_block {
            self.code_buffer.push(line.to_string());
            if self.code_is_asset {
                String::new()
            } else {
                format!("{}\n", highlight_code_line(&self.code_lang, line))
            }
        } else if line.trim() == "$$" {
            if self.in_math_block {
                self.in_math_block = false;
                let output = asset_block::render_math_block(&self.math_buffer);
                self.math_buffer.clear();
                output
            } else {
                let pending = self.flush();
                self.in_math_block = true;
                self.math_buffer.clear();
                pending
            }
        } else if self.in_math_block {
            self.math_buffer.push(line.to_string());
            String::new()
        } else if table::looks_like_table_row(line) {
            self.table_buffer.push(line.to_string());
            String::new()
        } else {
            let mut output = self.flush();
            output.push_str(&render_markdown_line(line));
            output.push('\n');
            output
        }
    }

    /// 刷新行级渲染器缓冲。
    ///
    /// 返回:
    /// - 缓冲区渲染结果
    fn flush(&mut self) -> String {
        if self.in_code_block {
            self.in_code_block = false;
            let lang = std::mem::take(&mut self.code_lang);
            let lines = std::mem::take(&mut self.code_buffer);
            if self.code_is_asset {
                asset_block::render_asset_block(&lang, &lines)
            } else {
                render_code_footer(&lines)
            }
        } else if self.in_math_block {
            self.in_math_block = false;
            let output = asset_block::render_math_block(&self.math_buffer);
            self.math_buffer.clear();
            output
        } else if self.table_buffer.is_empty() {
            String::new()
        } else {
            let lines = std::mem::take(&mut self.table_buffer);
            if lines.len() >= 2
                && table::is_table_separator(lines.get(1).map(String::as_str).unwrap_or(""))
            {
                table::render_table(&lines, render_table_cell_content)
            } else {
                let mut output = String::new();
                for line in lines {
                    output.push_str(&render_markdown_line(&line));
                    output.push('\n');
                }
                output
            }
        }
    }
}

/// 渲染单行 Markdown 文本。
///
/// 参数:
/// - `line`: 原始 Markdown 行
///
/// 返回:
/// - 渲染后的终端文本，不包含结尾换行
pub(crate) fn render_markdown_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];
    if let Some(header) = render_header(trimmed) {
        return header;
    }
    if let Some((depth, rest)) = parse_blockquote(trimmed) {
        let bars = "\x1b[32m| \x1b[0m".repeat(depth);
        return format!("{indent}{bars}\x1b[32m{}\x1b[0m", render_inline(rest));
    }
    if let Some(rest) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))
    {
        return format!("{indent}{TERTIARY_STYLE}-{RESET} {}", render_inline(rest));
    }
    let digits = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count();
    if digits > 0
        && trimmed.as_bytes().get(digits) == Some(&b'.')
        && trimmed.as_bytes().get(digits + 1) == Some(&b' ')
    {
        let marker = &trimmed[..=digits];
        let rest = &trimmed[digits + 2..];
        return format!(
            "{indent}{TERTIARY_STYLE}{marker}{RESET} {}",
            render_inline(rest)
        );
    }
    if markdown_blocks::is_horizontal_rule(trimmed) {
        return markdown_blocks::horizontal_rule();
    }
    render_inline(line)
}

/// 解析 Markdown 引用层级。
///
/// 参数:
/// - `line`: 原始行
///
/// 返回:
/// - 引用层级和剩余文本
fn parse_blockquote(line: &str) -> Option<(usize, &str)> {
    let mut depth = 0;
    let mut rest = line;
    while let Some(stripped) = rest.strip_prefix('>') {
        depth += 1;
        rest = stripped.strip_prefix(' ').unwrap_or(stripped);
    }
    (depth > 0).then_some((depth, rest))
}

/// 渲染 Markdown 标题。
///
/// 参数:
/// - `line`: 去除缩进后的行
///
/// 返回:
/// - 标题渲染结果
fn render_header(line: &str) -> Option<String> {
    let level = line.chars().take_while(|ch| *ch == '#').count();
    if level == 0 || level > 6 || line.as_bytes().get(level) != Some(&b' ') {
        return None;
    }
    let prefix = "#".repeat(level);
    Some(format!(
        "{HEADER_STYLE}{prefix} {}{RESET}",
        render_inline(&line[level + 1..])
    ))
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
