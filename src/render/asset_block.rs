use crate::render::style::{ASSET_ERROR_STYLE, INLINE_CODE_STYLE, RESET};
use crate::render::table::CellContent;
use crate::render::terminal_image;
use anyhow::{bail, Context, Result};
use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_render::{render_to_png, RenderOptions};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

#[derive(Clone, Copy)]
enum AssetKind {
    Mermaid,
    Math,
}

#[derive(Clone, Copy)]
enum MathRenderMode {
    Block,
    Inline,
}

impl AssetKind {
    /// 返回资产类型展示名称。
    ///
    /// 返回:
    /// - 展示名称
    fn label(self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::Math => "math",
        }
    }
}

/// 判断代码块语言是否需要渲染为图片资产。
///
/// 参数:
/// - `lang`: Markdown 代码块语言
///
/// 返回:
/// - 是否需要走图片资产渲染
pub(crate) fn is_asset_language(lang: &str) -> bool {
    asset_kind_from_lang(lang).is_some()
}

/// 渲染 Markdown 资产代码块。
///
/// 参数:
/// - `lang`: Markdown 代码块语言
/// - `lines`: 代码块内容行
///
/// 返回:
/// - 终端图片协议文本或错误提示
pub(crate) fn render_asset_block(lang: &str, lines: &[String]) -> String {
    let Some(kind) = asset_kind_from_lang(lang) else {
        return render_error("asset", "unsupported asset language");
    };
    render_asset(kind, &lines.join("\n"))
}

/// 渲染 `$$` 数学块。
///
/// 参数:
/// - `lines`: 数学公式内容行
///
/// 返回:
/// - 终端图片协议文本或错误提示
pub(crate) fn render_math_block(lines: &[String]) -> String {
    render_math_source(&lines.join("\n"), MathRenderMode::Block)
}

/// 渲染行内数学公式。
///
/// 参数:
/// - `source`: 数学公式源码
///
/// 返回:
/// - 终端图片协议文本或错误提示
pub(crate) fn render_inline_math(source: &str) -> String {
    render_math_source(source, MathRenderMode::Inline)
}

/// 渲染行内数学公式为单行半块图片，用于表格单元格。
///
/// 参数:
/// - `source`: 数学公式源码
///
/// 返回:
/// - 单行终端半块文本，失败时回退为带样式源码
pub(crate) fn render_inline_math_halfblock(source: &str) -> String {
    if source.trim().is_empty() {
        return String::new();
    }
    match render_inline_math_halfblock_inner(source) {
        Ok(rendered) => rendered,
        Err(_) => {
            let mut fallback = String::new();
            fallback.push_str(INLINE_CODE_STYLE);
            fallback.push_str(source);
            fallback.push_str(RESET);
            fallback
        }
    }
}

/// 生成行内公式图片并转换为单行半块文本。
fn render_inline_math_halfblock_inner(source: &str) -> Result<String> {
    let temp_dir = tempfile::tempdir().context("failed to create temporary render directory")?;
    let png = render_math_image(source, &temp_dir, MathRenderMode::Inline)?;
    terminal_image::render_halfblock_line(&png, 8)
}

/// 渲染表格内含公式单元格为终端图片协议。
///
/// 参数:
/// - `source`: 纯公式源码，或文字+公式混合原文
/// - `max_cols`: 最大终端列宽；`usize::MAX` 表示按终端估算
/// - `mixed`: 是否为文字+公式混合内容
///
/// 返回:
/// - 图片协议单元格内容，失败时回退为带样式源码
pub(crate) fn render_inline_math_table_cell(
    source: &str,
    max_cols: usize,
    mixed: bool,
) -> CellContent {
    if source.trim().is_empty() {
        return CellContent::empty();
    }
    match render_inline_math_table_cell_inner(source, max_cols, mixed) {
        Ok(mut content) => {
            // 二次适配时保留源码与混合标记
            content.math_source = Some(encode_table_math_source(source, mixed));
            content
        }
        Err(_) => {
            let mut fallback = String::new();
            fallback.push_str(INLINE_CODE_STYLE);
            fallback.push_str(source);
            fallback.push_str(RESET);
            CellContent::from_inline(fallback)
        }
    }
}

/// 编码表格公式源，供列宽二次适配还原。
///
/// 参数:
/// - `source`: 渲染源码
/// - `mixed`: 是否混合内容
///
/// 返回:
/// - 带前缀的源码标记
fn encode_table_math_source(source: &str, mixed: bool) -> String {
    if mixed {
        format!("mixed:{source}")
    } else {
        format!("pure:{source}")
    }
}

/// 解码表格公式源标记。
///
/// 参数:
/// - `encoded`: `encode_table_math_source` 的结果
///
/// 返回:
/// - `(源码, 是否混合)`
pub(crate) fn decode_table_math_source(encoded: &str) -> (String, bool) {
    if let Some(rest) = encoded.strip_prefix("mixed:") {
        return (rest.to_string(), true);
    }
    if let Some(rest) = encoded.strip_prefix("pure:") {
        return (rest.to_string(), false);
    }
    (encoded.to_string(), false)
}

/// 生成公式 PNG 并以图片协议渲染为表格单元格。
///
/// 参数:
/// - `source`: 公式或混合原文
/// - `max_cols`: 最大列宽
/// - `mixed`: 是否混合内容
///
/// 返回:
/// - 图片协议单元格
fn render_inline_math_table_cell_inner(
    source: &str,
    max_cols: usize,
    mixed: bool,
) -> Result<CellContent> {
    let temp_dir = tempfile::tempdir().context("failed to create temporary render directory")?;
    let png = if mixed {
        render_mixed_cell_math_image(source, &temp_dir)?
    } else {
        render_math_image(source, &temp_dir, MathRenderMode::Inline)?
    };
    let (term_cols, _) = crossterm::terminal::size()
        .map(|(cols, rows)| (usize::from(cols), usize::from(rows)))
        .unwrap_or((80, 24));
    let default_max = (term_cols.saturating_sub(10) * 2 / 5).clamp(8, 36);
    let max_cols = if max_cols == usize::MAX {
        default_max
    } else {
        max_cols.clamp(1, 48)
    };
    terminal_image::render_inline_image_with_max_cols(&png, max_cols)
}

/// 渲染表格混合单元格（文字 + 公式）为 PNG。
///
/// 参数:
/// - `source`: 含 `$...$` 的单元格原文
/// - `temp_dir`: 临时目录
///
/// 返回:
/// - PNG 路径
fn render_mixed_cell_math_image(source: &str, temp_dir: &TempDir) -> Result<PathBuf> {
    // 1. 优先 Typst：原生支持中文与行内公式混排
    if let Some(output) = try_render_typst_mixed_cell(source, temp_dir)? {
        return Ok(output);
    }
    // 2. 转成 LaTeX 后走 RaTeX
    let latex = mixed_cell_to_latex(source);
    if let Some(output) = try_render_ratex_math(&latex, temp_dir, MathRenderMode::Inline)? {
        return Ok(output);
    }
    // 3. 降级 SVG 文本图
    let svg = temp_dir.path().join("formula.svg");
    let output = temp_dir.path().join("formula.png");
    fs::write(&svg, build_math_svg(source, MathRenderMode::Inline))
        .with_context(|| format!("failed to write {}", svg.display()))?;
    convert_svg_to_png(&svg, &output)?;
    ensure_file_exists(&output)?;
    Ok(output)
}

/// 将文字+公式单元格转为 LaTeX 源码。
///
/// 参数:
/// - `source`: 单元格原文
///
/// 返回:
/// - `\text{...}` 与公式拼接后的 LaTeX
fn mixed_cell_to_latex(source: &str) -> String {
    let chars = source.chars().collect::<Vec<_>>();
    let mut output = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        if index + 1 < chars.len() && chars[index] == '$' && chars[index + 1] == '$' {
            if let Some(end) = find_math_end(&chars, index + 2, true) {
                let formula = chars[index + 2..end].iter().collect::<String>();
                output.push_str(formula.trim());
                index = end + 2;
                continue;
            }
        }
        if chars[index] == '$' {
            if let Some(end) = find_math_end(&chars, index + 1, false) {
                let formula = chars[index + 1..end].iter().collect::<String>();
                output.push_str(formula.trim());
                index = end + 1;
                continue;
            }
        }
        let start = index;
        while index < chars.len() && chars[index] != '$' {
            index += 1;
        }
        let text = chars[start..index].iter().collect::<String>();
        if !text.is_empty() {
            let escaped = text
                .replace('\\', "\\textbackslash{}")
                .replace('{', "\\{")
                .replace('}', "\\}");
            output.push_str("\\text{");
            output.push_str(&escaped);
            output.push('}');
        }
    }
    output
}

/// 在字符切片中查找公式结束的 `$` 位置。
///
/// 参数:
/// - `chars`: 字符切片
/// - `start`: 公式内容起始下标
/// - `display`: 是否为 `$$` 显示公式
///
/// 返回:
/// - 结束标记起始下标
fn find_math_end(chars: &[char], start: usize, display: bool) -> Option<usize> {
    let mut index = start;
    while index < chars.len() {
        if display {
            if chars[index] == '$' && chars.get(index + 1) == Some(&'$') {
                return Some(index);
            }
        } else if chars[index] == '$' {
            return Some(index);
        }
        index += 1;
    }
    None
}

/// 使用 Typst 渲染文字+公式混合单元格。
///
/// 参数:
/// - `source`: 单元格原文（保留 `$...$`）
/// - `temp_dir`: 临时目录
///
/// 返回:
/// - 成功时返回 PNG 路径
fn try_render_typst_mixed_cell(source: &str, temp_dir: &TempDir) -> Result<Option<PathBuf>> {
    if !command_available("typst") {
        return Ok(None);
    }
    let input = temp_dir.path().join("mixed_cell.typ");
    let output = temp_dir.path().join("mixed_cell.png");
    let body = escape_typst_mixed_cell(source);
    let content = format!(
        "#set page(width: auto, height: auto, margin: 2pt)\n#set text(fill: rgb(\"d7e3ff\"), size: 11pt)\n{body}\n"
    );
    fs::write(&input, content).with_context(|| format!("failed to write {}", input.display()))?;
    let mut command = Command::new("typst");
    command
        .arg("compile")
        .arg(&input)
        .arg(&output)
        .stdin(Stdio::null());
    if run_command(command, "typst").is_ok() && output.is_file() {
        return Ok(Some(output));
    }
    Ok(None)
}

/// 转义 Typst 混合单元格中的特殊字符，保留 `$...$` 公式。
///
/// 参数:
/// - `source`: 单元格原文
///
/// 返回:
/// - 可写入 Typst 文档的正文
fn escape_typst_mixed_cell(source: &str) -> String {
    let chars = source.chars().collect::<Vec<_>>();
    let mut output = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        if index + 1 < chars.len() && chars[index] == '$' && chars[index + 1] == '$' {
            if let Some(end) = find_math_end(&chars, index + 2, true) {
                output.push_str("$ ");
                output.extend(chars[index + 2..end].iter());
                output.push_str(" $");
                index = end + 2;
                continue;
            }
        }
        if chars[index] == '$' {
            if let Some(end) = find_math_end(&chars, index + 1, false) {
                output.push('$');
                output.extend(chars[index + 1..end].iter());
                output.push('$');
                index = end + 1;
                continue;
            }
        }
        match chars[index] {
            '#' => output.push_str("\\#"),
            '\\' => output.push_str("\\\\"),
            '*' => output.push_str("\\*"),
            '_' => output.push_str("\\_"),
            '`' => output.push_str("\\`"),
            '<' => output.push_str("\\<"),
            '>' => output.push_str("\\>"),
            '@' => output.push_str("\\@"),
            other => output.push(other),
        }
        index += 1;
    }
    output
}

/// 解析资产代码块类型。
///
/// 参数:
/// - `lang`: Markdown 代码块语言
///
/// 返回:
/// - 资产类型
fn asset_kind_from_lang(lang: &str) -> Option<AssetKind> {
    match lang.trim().to_ascii_lowercase().as_str() {
        "mermaid" | "mmd" => Some(AssetKind::Mermaid),
        "math" | "latex" | "tex" => Some(AssetKind::Math),
        _ => None,
    }
}

/// 渲染单个图片资产。
///
/// 参数:
/// - `kind`: 资产类型
/// - `source`: 原始内容
///
/// 返回:
/// - 终端图片协议文本或错误提示
fn render_asset(kind: AssetKind, source: &str) -> String {
    if source.trim().is_empty() {
        return render_error(kind.label(), "content is empty");
    }
    if matches!(kind, AssetKind::Math) {
        return render_math_source(source, MathRenderMode::Block);
    }
    if test_stub_enabled() {
        return render_success("[asset rendering skipped]\n".to_string());
    }
    match render_asset_inner(kind, source) {
        Ok(rendered) => render_success(rendered),
        Err(error) => render_error(kind.label(), &error.to_string()),
    }
}

/// 渲染数学公式源码。
///
/// 参数:
/// - `source`: 数学公式源码
/// - `mode`: 数学公式展示模式
///
/// 返回:
/// - 终端图片协议文本或错误提示
fn render_math_source(source: &str, mode: MathRenderMode) -> String {
    if source.trim().is_empty() {
        return render_error("math", "content is empty");
    }
    if test_stub_enabled() {
        let placeholder = match mode {
            MathRenderMode::Block => "[asset rendering skipped]\n".to_string(),
            MathRenderMode::Inline => "[inline math rendering skipped]\n".to_string(),
        };
        return match mode {
            MathRenderMode::Block => render_success(placeholder),
            MathRenderMode::Inline => placeholder,
        };
    }
    match render_math_inner(source, mode) {
        Ok(rendered) => match mode {
            MathRenderMode::Block => render_success(rendered),
            MathRenderMode::Inline => rendered,
        },
        Err(error) => render_error("math", &error.to_string()),
    }
}

/// 生成数学公式临时图片并转换为终端图片文本。
///
/// 参数:
/// - `source`: 数学公式源码
/// - `mode`: 数学公式展示模式
///
/// 返回:
/// - 终端图片协议文本或 chafa 文本输出
fn render_math_inner(source: &str, mode: MathRenderMode) -> Result<String> {
    let temp_dir = tempfile::tempdir().context("failed to create temporary render directory")?;
    let image = render_math_image(source, &temp_dir, mode)?;
    terminal_image::render_terminal_image(&image)
}

/// 生成资产临时图片并转换为终端图片文本。
///
/// 参数:
/// - `kind`: 资产类型
/// - `source`: 原始内容
///
/// 返回:
/// - 终端图片协议文本或 chafa 文本输出
fn render_asset_inner(kind: AssetKind, source: &str) -> Result<String> {
    let temp_dir = tempfile::tempdir().context("failed to create temporary render directory")?;
    let image = match kind {
        AssetKind::Mermaid => render_mermaid_image(source, &temp_dir)?,
        AssetKind::Math => render_math_image(source, &temp_dir, MathRenderMode::Block)?,
    };
    terminal_image::render_terminal_image(&image)
}

/// 使用纯 Rust Mermaid 渲染器生成图表图片。
///
/// 参数:
/// - `source`: Mermaid 源码
/// - `temp_dir`: 临时目录
///
/// 返回:
/// - 生成的 PNG 路径
fn render_mermaid_image(source: &str, temp_dir: &TempDir) -> Result<PathBuf> {
    let output = temp_dir.path().join("diagram.png");
    let theme = transparent_mermaid_theme();
    let svg = mermaid_rs_renderer::render_with_options(
        source,
        mermaid_rs_renderer::RenderOptions {
            theme: theme.clone(),
            layout: mermaid_rs_renderer::LayoutConfig::default(),
        },
    )
    .context("failed to render mermaid svg")?;
    mermaid_rs_renderer::write_output_png(
        &svg,
        &output,
        &mermaid_rs_renderer::RenderConfig::default(),
        &theme,
    )
    .with_context(|| format!("failed to write {}", output.display()))?;
    ensure_file_exists(&output)?;
    Ok(output)
}

/// 返回透明画布的 Mermaid 主题。
///
/// 返回:
/// - Mermaid 渲染主题
fn transparent_mermaid_theme() -> mermaid_rs_renderer::Theme {
    let mut theme = mermaid_rs_renderer::Theme::modern();
    theme.background = "transparent".to_string();
    theme
}

/// 生成数学公式图片。
///
/// 参数:
/// - `source`: 数学公式源码
/// - `temp_dir`: 临时目录
///
/// 返回:
/// - 生成的 PNG 路径
fn render_math_image(source: &str, temp_dir: &TempDir, mode: MathRenderMode) -> Result<PathBuf> {
    if let Some(output) = try_render_ratex_math(source, temp_dir, mode)? {
        return Ok(output);
    }
    if let Some(output) = try_render_typst_math(source, temp_dir, mode)? {
        return Ok(output);
    }
    let svg = temp_dir.path().join("formula.svg");
    let output = temp_dir.path().join("formula.png");
    fs::write(&svg, build_math_svg(source, mode))
        .with_context(|| format!("failed to write {}", svg.display()))?;
    convert_svg_to_png(&svg, &output)?;
    ensure_file_exists(&output)?;
    Ok(output)
}

/// 优先使用 RaTeX 的纯 Rust 管线渲染数学公式。
///
/// 参数:
/// - `source`: 数学公式源码
/// - `temp_dir`: 临时目录
///
/// 返回:
/// - 成功时返回图片路径，失败时返回空并交给后续降级方案
fn try_render_ratex_math(
    source: &str,
    temp_dir: &TempDir,
    mode: MathRenderMode,
) -> Result<Option<PathBuf>> {
    let formula = normalize_math_source(source);
    let ast = match parse(&formula) {
        Ok(ast) => ast,
        Err(_) => return Ok(None),
    };
    let color = Color::parse("#d7e3ff").unwrap_or(Color::BLACK);
    let background_color = transparent_color();
    let math_style = match mode {
        MathRenderMode::Block => MathStyle::Display,
        MathRenderMode::Inline => MathStyle::Text,
    };
    let (font_size, padding, device_pixel_ratio) = match mode {
        MathRenderMode::Block => (28.0, 4.0, 1.5),
        MathRenderMode::Inline => (18.0, 0.0, 1.5),
    };
    let layout_opts = LayoutOptions::default()
        .with_style(math_style)
        .with_color(color);
    let render_opts = RenderOptions {
        font_size,
        padding,
        background_color,
        font_dir: String::new(),
        device_pixel_ratio,
    };
    let layout_box = layout(&ast, &layout_opts);
    let display_list = to_display_list(&layout_box);
    let png = match render_to_png(&display_list, &render_opts) {
        Ok(png) => png,
        Err(_) => return Ok(None),
    };
    let output = temp_dir.path().join("formula.png");
    fs::write(&output, png).with_context(|| format!("failed to write {}", output.display()))?;
    ensure_file_exists(&output)?;
    Ok(Some(output))
}

/// 返回透明背景颜色。
///
/// 返回:
/// - 透明颜色
fn transparent_color() -> Color {
    Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    }
}

/// 归一化数学公式源码。
///
/// 参数:
/// - `source`: 原始公式
///
/// 返回:
/// - 适合数学解析器处理的单行公式
fn normalize_math_source(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 优先尝试用 Typst 渲染数学公式。
///
/// 参数:
/// - `source`: 数学公式源码
/// - `temp_dir`: 临时目录
/// - `mode`: 数学公式展示模式
///
/// 返回:
/// - 成功时返回图片路径，未启用时返回空
fn try_render_typst_math(
    source: &str,
    temp_dir: &TempDir,
    mode: MathRenderMode,
) -> Result<Option<PathBuf>> {
    if !command_available("typst") {
        return Ok(None);
    }
    let input = temp_dir.path().join("formula.typ");
    let output = temp_dir.path().join("formula.png");
    let (margin, text_size) = match mode {
        MathRenderMode::Block => (6, 14),
        MathRenderMode::Inline => (2, 11),
    };
    let content = format!(
        "#set page(width: auto, height: auto, margin: {margin}pt)\n#set text(fill: rgb(\"d7e3ff\"), size: {text_size}pt)\n$ {} $\n",
        source.trim()
    );
    fs::write(&input, content).with_context(|| format!("failed to write {}", input.display()))?;
    let mut command = Command::new("typst");
    command
        .arg("compile")
        .arg(&input)
        .arg(&output)
        .stdin(Stdio::null());
    if run_command(command, "typst").is_ok() && output.is_file() {
        return Ok(Some(output));
    }
    Ok(None)
}

/// 将 SVG 公式图片转换为 PNG。
///
/// 参数:
/// - `svg`: SVG 输入路径
/// - `output`: PNG 输出路径
///
/// 返回:
/// - 转换是否成功
fn convert_svg_to_png(svg: &Path, output: &Path) -> Result<()> {
    let mut rsvg = Command::new("rsvg-convert");
    rsvg.arg("-f")
        .arg("png")
        .arg("-o")
        .arg(output)
        .arg(svg)
        .stdin(Stdio::null());
    if run_command(rsvg, "rsvg-convert").is_ok() {
        return Ok(());
    }
    let mut magick = Command::new("magick");
    magick.arg(svg).arg(output).stdin(Stdio::null());
    run_command(magick, "magick")
}

/// 构建数学公式降级 SVG。
///
/// 参数:
/// - `source`: 数学公式源码
/// - `mode`: 数学公式展示模式
///
/// 返回:
/// - SVG 文本
fn build_math_svg(source: &str, mode: MathRenderMode) -> String {
    let lines = wrap_math_lines(source.trim(), 72);
    let (font_size, char_width, line_height, side_padding, top_padding) = match mode {
        MathRenderMode::Block => (17, 9, 24, 28, 30),
        MathRenderMode::Inline => (13, 7, 18, 12, 20),
    };
    let max_chars = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1);
    let min_width = match mode {
        MathRenderMode::Block => 180,
        MathRenderMode::Inline => 40,
    };
    let min_height = match mode {
        MathRenderMode::Block => 48,
        MathRenderMode::Inline => 24,
    };
    let width = (max_chars * char_width + side_padding * 2).clamp(min_width, 1200);
    let height = (lines.len() * line_height + top_padding).clamp(min_height, 1200);
    let mut text = String::new();
    for (index, line) in lines.iter().enumerate() {
        let y = top_padding + index * line_height;
        text.push_str(&format!(
            r##"<text x="{side_padding}" y="{y}" font-family="DejaVu Sans Mono, monospace" font-size="{font_size}" fill="#d7e3ff">{}</text>"##,
            escape_xml(line)
        ));
    }
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">{text}</svg>"##
    )
}

/// 按字符数量拆分数学公式行。
///
/// 参数:
/// - `source`: 原始公式
/// - `limit`: 每行最大字符数
///
/// 返回:
/// - 拆分后的公式行
fn wrap_math_lines(source: &str, limit: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in source.lines() {
        let mut current = String::new();
        for ch in raw_line.chars() {
            current.push(ch);
            if current.chars().count() >= limit {
                lines.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(source.to_string());
    }
    lines
}

/// 转义 XML 文本。
///
/// 参数:
/// - `text`: 原始文本
///
/// 返回:
/// - 可安全写入 XML 的文本
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// 执行外部渲染命令。
///
/// 参数:
/// - `command`: 待执行命令
/// - `name`: 命令名称
///
/// 返回:
/// - 命令是否执行成功
fn run_command(mut command: Command, name: &str) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("failed to run {name}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let message = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        bail!("{name} exited with status {}: {message}", output.status);
    }
    Ok(())
}

/// 判断命令是否存在。
///
/// 参数:
/// - `name`: 命令名称
///
/// 返回:
/// - 命令是否可执行
fn command_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// 确认输出文件已经生成。
///
/// 参数:
/// - `path`: 输出文件路径
///
/// 返回:
/// - 文件是否存在
fn ensure_file_exists(path: &Path) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        bail!("renderer did not create {}", path.display())
    }
}

/// 渲染成功图片文本。
///
/// 参数:
/// - `rendered`: 图片渲染文本
///
/// 返回:
/// - 图片文本
fn render_success(rendered: String) -> String {
    rendered
}

/// 渲染资产错误提示。
///
/// 参数:
/// - `label`: 资产类型标签
/// - `message`: 错误信息
///
/// 返回:
/// - 带样式的错误提示
fn render_error(label: &str, message: &str) -> String {
    format!("{ASSET_ERROR_STYLE}[{label} render failed: {message}]{RESET}\n")
}

/// 判断测试替身是否开启。
///
/// 返回:
/// - 是否开启测试替身
fn test_stub_enabled() -> bool {
    cfg!(test) && std::env::var_os("MIYU_RENDER_ASSET_TEST_STUB").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_asset_languages() {
        assert!(is_asset_language("mermaid"));
        assert!(is_asset_language("math"));
        assert!(is_asset_language("latex"));
        assert!(!is_asset_language("rust"));
    }

    #[test]
    fn math_svg_escapes_formula_text() {
        let svg = build_math_svg(r#"a < b && c > "d""#, MathRenderMode::Block);
        assert!(svg.contains("&lt;"));
        assert!(svg.contains("&gt;"));
        assert!(svg.contains("&quot;"));
    }

    #[test]
    fn ratex_renders_common_formulas_to_png() {
        let temp_dir = tempfile::tempdir().unwrap();
        let formulas = [
            r"\int_a^b f(x)\,dx = F(b) - F(a)",
            r"\lim_{x \to 0} \frac{\sin x}{x} = 1",
            r"\left( \sum_{i=1}^{n} a_i b_i \right)^2 \le \left( \sum_{i=1}^{n} a_i^2 \right) \left( \sum_{i=1}^{n} b_i^2 \right)",
            r"P(A|B) = \frac{P(B|A)\,P(A)}{P(B)}",
            r"\mathcal{L}\{f(t)\} = F(s) = \int_0^\infty e^{-st} f(t)\,dt",
        ];
        for formula in formulas {
            let output = try_render_ratex_math(formula, &temp_dir, MathRenderMode::Block).unwrap();
            let output = output.expect("RaTeX should render this formula");
            let metadata = fs::metadata(output).unwrap();
            assert!(metadata.len() > 0);
        }
    }

    #[test]
    fn ratex_renders_inline_formula_to_png() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output = try_render_ratex_math(
            r"\lim_{x \to 0} \frac{\sin x}{x} = 1",
            &temp_dir,
            MathRenderMode::Inline,
        )
        .unwrap();
        let output = output.expect("RaTeX should render inline formula");
        let metadata = fs::metadata(output).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn mermaid_renders_to_png_without_external_cli() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output = render_mermaid_image("graph TD\nA[Start] --> B[End]", &temp_dir).unwrap();
        let metadata = fs::metadata(output).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn mermaid_png_preserves_transparent_canvas() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output = render_mermaid_image("graph TD\nA[Start] --> B[End]", &temp_dir).unwrap();
        assert!(png_has_transparent_pixel(&output));
    }

    fn png_has_transparent_pixel(path: &Path) -> bool {
        let file = fs::File::open(path).unwrap();
        let decoder = png::Decoder::new(std::io::BufReader::new(file));
        let mut reader = decoder.read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buffer).unwrap();
        let bytes = &buffer[..info.buffer_size()];
        match (info.color_type, info.bit_depth) {
            (png::ColorType::Rgba, png::BitDepth::Eight) => {
                bytes.chunks_exact(4).any(|chunk| chunk[3] == 0)
            }
            (png::ColorType::Rgba, png::BitDepth::Sixteen) => {
                bytes.chunks_exact(8).any(|chunk| chunk[6] == 0)
            }
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight) => {
                bytes.chunks_exact(2).any(|chunk| chunk[1] == 0)
            }
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Sixteen) => {
                bytes.chunks_exact(4).any(|chunk| chunk[2] == 0)
            }
            _ => false,
        }
    }
}
