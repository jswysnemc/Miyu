/// 使用 Sixel 图形协议渲染图片。
///
/// 参数:
/// - `path`: 图片文件路径
///
/// 返回:
/// - Sixel 图形协议转义序列
fn render_sixel_image(path: &Path, size: &TerminalImageSize) -> Result<String> {
    let image = load_image_rgba(path)?;
    let (width, height) = sixel_dimensions(image.width, image.height, size);
    let pixels = quantize_for_sixel(&image, width, height);
    Ok(encode_sixel(&pixels, width, height))
}

/// 使用 chafa 降级渲染图片。
///
/// 参数:
/// - `path`: 图片文件路径
///
/// 返回:
/// - chafa 输出的终端文本
fn render_chafa_image(
    path: &Path,
    raw_size: Option<&str>,
    parsed_size: &TerminalImageSize,
) -> Result<String> {
    let mut command = Command::new("chafa");
    if let Some(size) = raw_size
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| chafa_size(parsed_size))
    {
        command.arg("--size").arg(size);
    }
    command.arg("--fg-only").arg("--threshold").arg("0.75");
    run_chafa(
        command,
        path,
        "failed to run chafa; install chafa or use a terminal image protocol",
    )
}

/// 执行 chafa 并返回标准输出文本。
///
/// 参数:
/// - `command`: 已配置的 chafa 命令
/// - `path`: 图片文件路径
/// - `context`: 命令执行失败时的上下文
///
/// 返回:
/// - chafa 输出文本
fn run_chafa(mut command: Command, path: &Path, context: &str) -> Result<String> {
    let output = command
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .with_context(|| context.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "chafa exited with status {}: {}",
            output.status,
            stderr.trim()
        );
    }
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// 使用 ANSI 真彩色半块字符渲染 PNG，作为不依赖外部命令的兜底方案。
///
/// 参数:
/// - `path`: PNG 图片路径
///
/// 返回:
/// - 可直接打印到终端的 ANSI 文本
fn render_ansi_halfblock_image(path: &Path, size: &TerminalImageSize) -> Result<String> {
    let image = load_image_rgba(path)?;
    let (width, height) = ansi_dimensions(image.width, image.height, size);
    let mut output = String::new();
    let reset = "\x1b[0m";
    for y in (0..height).step_by(2) {
        for x in 0..width {
            let top = sample_resized_pixel(&image, x, y, width, height);
            let bottom = if y + 1 < height {
                sample_resized_pixel(&image, x, y + 1, width, height)
            } else {
                Rgba { a: 0, ..top }
            };
            output.push_str(&render_halfblock_cell(top, bottom));
        }
        output.push_str(reset);
        output.push('\n');
    }
    Ok(output)
}

/// 将 PNG 渲染为单终端行半块图片，尾部补齐空格以匹配显示宽度。
///
/// 参数:
/// - `path`: PNG 图片路径
/// - `target_height_px`: 目标像素高度（应为 2 的倍数）
///
/// 返回:
/// - 单行 ANSI 半块文本，宽度由图片比例决定
pub(crate) fn render_halfblock_line(path: &Path, target_height_px: usize) -> Result<String> {
    let image = load_image_rgba(path)?;
    let target_height_px = target_height_px.max(2);
    let target_width = if image.height == 0 {
        1
    } else {
        (image.width as usize * target_height_px / image.height as usize).max(1)
    };
    let mut output = String::new();
    let reset = "\x1b[0m";
    for x in 0..target_width {
        let top = sample_resized_pixel(&image, x, 0, target_width, target_height_px);
        let bottom = if target_height_px > 1 {
            sample_resized_pixel(&image, x, 1, target_width, target_height_px)
        } else {
            Rgba { a: 0, ..top }
        };
        output.push_str(&render_halfblock_cell(top, bottom));
    }
    output.push_str(reset);
    Ok(output)
}

/// 将 PNG 按当前终端图片协议渲染，并返回已知终端单元格尺寸。
///
/// 优先使用 Kitty 图形协议，其次 iTerm2，最后 Sixel。
///
/// 参数:
/// - `path`: PNG 图片路径
///
/// 返回:
/// - 包含图片协议文本和终端宽度的单元格内容
pub(crate) fn render_inline_image_with_cell_size(path: &Path) -> Result<CellContent> {
    let image = load_image_rgba(path)?;
    let (cell_pw, cell_ph) = terminal_cell_pixel_size();
    let cell_width = ((image.width + cell_pw - 1) / cell_pw).clamp(1, 30);
    let cell_height = ((image.height + cell_ph - 1) / cell_ph).clamp(1, 4);
    let (pixel_width, pixel_height) =
        sixel_dimensions(image.width, image.height, &TerminalImageSize::default());

    if supports_kitty_graphics() {
        let kitty = render_kitty_image(path)?;
        let kitty = kitty.trim_end().to_string();
        let mut lines = vec![kitty];
        for _ in 1..cell_height {
            lines.push(String::new());
        }
        return Ok(CellContent {
            lines,
            width: cell_width,
            is_image: true,
        });
    }
    if supports_iterm_inline_image() {
        let iterm = render_iterm_image(path)?;
        let iterm = iterm.trim_end().to_string();
        let mut lines = vec![iterm];
        for _ in 1..cell_height {
            lines.push(String::new());
        }
        return Ok(CellContent {
            lines,
            width: cell_width,
            is_image: true,
        });
    }
    let pixels = quantize_for_sixel(&image, pixel_width, pixel_height);
    let sixel = encode_sixel(&pixels, pixel_width, pixel_height);
    let sixel = sixel.trim_end().to_string();
    let mut lines = vec![sixel];
    for _ in 1..cell_height {
        lines.push(String::new());
    }
    Ok(CellContent {
        lines,
        width: cell_width,
        is_image: true,
    })
}
