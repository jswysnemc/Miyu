use crate::render::table::CellContent;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose, Engine as _};
use crossterm::terminal;
use std::fs;
#[cfg(test)]
use std::fs::File;
#[cfg(test)]
use std::io::BufWriter;
use std::path::Path;
use std::process::{Command, Stdio};

const KITTY_CHUNK_SIZE: usize = 4096;
const ANSI_ALPHA_THRESHOLD: u8 = 16;
const SIXEL_CELL_WIDTH_PX: usize = 8;
const SIXEL_CELL_HEIGHT_PX: usize = 16;
const SIXEL_MAX_WIDTH_PX: usize = 1600;
const SIXEL_MAX_HEIGHT_PX: usize = 1200;
const SIXEL_COLOR_STEPS: [u8; 6] = [0, 51, 102, 153, 204, 255];
const ANSI_FALLBACK_BG: Rgba = Rgba {
    r: 11,
    g: 16,
    b: 32,
    a: 255,
};

/// 查询终端单元格的像素尺寸。
///
/// 通过 `ioctl(TIOCGWINSIZE)` 获取终端窗口总像素尺寸，
/// 除以字符行列数得到单个单元格的像素宽高。
/// 不支持时回退到 8×16。
#[cfg(unix)]
fn terminal_cell_pixel_size() -> (usize, usize) {
    use std::os::unix::io::AsRawFd;

    const TIOCGWINSIZE: libc::c_ulong = 0x5413;

    #[repr(C)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }

    let fd = std::io::stdout().as_raw_fd();
    let mut ws = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let ret = unsafe { libc::ioctl(fd, TIOCGWINSIZE, &mut ws) };
    if ret == 0 && ws.ws_col > 0 && ws.ws_row > 0 && ws.ws_xpixel > 0 && ws.ws_ypixel > 0 {
        let cw = ws.ws_xpixel as usize / ws.ws_col as usize;
        let ch = ws.ws_ypixel as usize / ws.ws_row as usize;
        if cw > 0 && ch > 0 {
            return (cw, ch);
        }
    }
    (SIXEL_CELL_WIDTH_PX, SIXEL_CELL_HEIGHT_PX)
}

#[cfg(not(unix))]
fn terminal_cell_pixel_size() -> (usize, usize) {
    (SIXEL_CELL_WIDTH_PX, SIXEL_CELL_HEIGHT_PX)
}

#[derive(Clone, Copy, Debug)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Clone, Copy, Debug, Default)]
struct TerminalImageSize {
    width_cells: Option<usize>,
    height_cells: Option<usize>,
}

/// 将图片渲染为当前终端可显示的文本。
///
/// 参数:
/// - `path`: 图片文件路径
///
/// 返回:
/// - 终端图片协议文本或 chafa 文本输出
pub(crate) fn render_terminal_image(path: &Path) -> Result<String> {
    render_terminal_image_with_size(path, None)
}

/// 将图片按可选终端单元格尺寸渲染为当前终端可显示的文本。
///
/// 参数:
/// - `path`: 图片文件路径
/// - `size`: 可选尺寸，格式同 chafa `WIDTHxHEIGHT`，允许省略一边
///
/// 返回:
/// - 终端图片协议文本、chafa 文本输出，或 ANSI 半块降级文本
pub(crate) fn render_terminal_image_with_size(path: &Path, size: Option<&str>) -> Result<String> {
    let parsed_size = TerminalImageSize::parse(size);
    if supports_kitty_graphics() {
        return render_kitty_image(path);
    }
    if supports_iterm_inline_image() {
        return render_iterm_image(path);
    }
    if supports_windows_terminal_sixel() {
        return render_sixel_image(path, &parsed_size)
            .or_else(|_| render_ansi_halfblock_image(path, &parsed_size));
    }
    render_chafa_image(path, size, &parsed_size)
        .or_else(|_| render_ansi_halfblock_image(path, &parsed_size))
}

impl TerminalImageSize {
    /// 解析终端图片尺寸。
    ///
    /// 参数:
    /// - `value`: `WIDTHxHEIGHT`、`WIDTHx` 或 `xHEIGHT`
    ///
    /// 返回:
    /// - 已解析的尺寸约束
    fn parse(value: Option<&str>) -> Self {
        let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
            return Self::default();
        };
        let Some((width, height)) = value.split_once('x') else {
            return Self::default();
        };
        Self {
            width_cells: parse_cell_count(width),
            height_cells: parse_cell_count(height),
        }
    }
}

/// 解析正整数终端单元格数量。
///
/// 参数:
/// - `value`: 数字文本
///
/// 返回:
/// - 有效正整数
fn parse_cell_count(value: &str) -> Option<usize> {
    value
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
}

/// 判断当前终端是否支持 Kitty 图形协议。
///
/// 返回:
/// - 是否支持 Kitty 图形协议
fn supports_kitty_graphics() -> bool {
    std::env::var_os("KITTY_WINDOW_ID").is_some()
        || std::env::var("TERM")
            .map(|term| term.contains("xterm-kitty"))
            .unwrap_or(false)
        || std::env::var("TERM_PROGRAM")
            .map(|program| program == "WezTerm")
            .unwrap_or(false)
}

/// 判断当前终端是否支持 iTerm2 图片协议。
///
/// 返回:
/// - 是否支持 iTerm2 图片协议
fn supports_iterm_inline_image() -> bool {
    std::env::var("TERM_PROGRAM")
        .map(|program| matches!(program.as_str(), "iTerm.app" | "WezTerm"))
        .unwrap_or(false)
}

/// 判断当前终端是否支持 Windows Terminal 使用的 Sixel 图形协议。
///
/// 返回:
/// - 是否可能支持 Sixel 图形协议
fn supports_windows_terminal_sixel() -> bool {
    std::env::var_os("WT_SESSION").is_some()
        || std::env::var("TERM_PROGRAM")
            .map(|program| program == "Windows_Terminal")
            .unwrap_or(false)
}

/// 使用 Kitty 图形协议渲染图片。
///
/// 参数:
/// - `path`: 图片文件路径
///
/// 返回:
/// - Kitty 图形协议转义序列
fn render_kitty_image(path: &Path) -> Result<String> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read image {}", path.display()))?;
    let encoded = general_purpose::STANDARD.encode(bytes);
    let mut output = String::new();
    let mut chunks = encoded.as_bytes().chunks(KITTY_CHUNK_SIZE).peekable();
    if let Some(first) = chunks.next() {
        let more = if chunks.peek().is_some() { 1 } else { 0 };
        output.push_str(&format!(
            "\x1b_Gf=100,a=T,m={more};{}\x1b\\",
            String::from_utf8_lossy(first)
        ));
    }
    while let Some(chunk) = chunks.next() {
        let more = if chunks.peek().is_some() { 1 } else { 0 };
        output.push_str(&format!(
            "\x1b_Gm={more};{}\x1b\\",
            String::from_utf8_lossy(chunk)
        ));
    }
    output.push('\n');
    Ok(output)
}

/// 使用 iTerm2 图片协议渲染图片。
///
/// 参数:
/// - `path`: 图片文件路径
///
/// 返回:
/// - iTerm2 图片协议转义序列
fn render_iterm_image(path: &Path) -> Result<String> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read image {}", path.display()))?;
    let encoded = general_purpose::STANDARD.encode(bytes);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image.png");
    let name = general_purpose::STANDARD.encode(name.as_bytes());
    Ok(format!(
        "\x1b]1337;File=inline=1;name={name}:{encoded}\x07\n"
    ))
}

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

struct RasterImage {
    pixels: Vec<Rgba>,
    width: usize,
    height: usize,
}

/// 解码常见图片格式为 RGBA 像素。
///
/// 参数:
/// - `path`: 图片路径
///
/// 返回:
/// - RGBA 图片数据
fn load_image_rgba(path: &Path) -> Result<RasterImage> {
    let image = image::ImageReader::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("failed to detect image format for {}", path.display()))?
        .decode()
        .with_context(|| format!("failed to decode image {}", path.display()))?;
    let rgba = image.to_rgba8();
    let width = usize::try_from(rgba.width()).context("image width is too large")?;
    let height = usize::try_from(rgba.height()).context("image height is too large")?;
    let pixels = rgba
        .pixels()
        .map(|pixel| Rgba {
            r: pixel[0],
            g: pixel[1],
            b: pixel[2],
            a: pixel[3],
        })
        .collect::<Vec<_>>();
    Ok(RasterImage {
        pixels,
        width,
        height,
    })
}

/// 将 PNG 像素缩放并量化为 sixel 可用的 216 色调色板索引。
///
/// 参数:
/// - `image`: 原图
/// - `target_width`: 目标宽度
/// - `target_height`: 目标高度
///
/// 返回:
/// - 每个目标像素的可选 sixel 调色板索引
fn quantize_for_sixel(
    image: &RasterImage,
    target_width: usize,
    target_height: usize,
) -> Vec<Option<u16>> {
    let mut pixels = Vec::with_capacity(target_width.saturating_mul(target_height));
    for y in 0..target_height {
        for x in 0..target_width {
            let pixel = sample_resized_pixel(image, x, y, target_width, target_height);
            if pixel.a < ANSI_ALPHA_THRESHOLD {
                pixels.push(None);
            } else {
                pixels.push(Some(sixel_palette_index(pixel)));
            }
        }
    }
    pixels
}

/// 把量化像素编码为 sixel DCS 文本。
///
/// 参数:
/// - `pixels`: 每个像素的可选调色板索引
/// - `width`: 图片宽度
/// - `height`: 图片高度
///
/// 返回:
/// - sixel 转义序列
fn encode_sixel(pixels: &[Option<u16>], width: usize, height: usize) -> String {
    let mut used_colors = [false; 216];
    for color in pixels.iter().flatten() {
        if let Some(used) = used_colors.get_mut(usize::from(*color)) {
            *used = true;
        }
    }

    let mut output = String::new();
    output.push_str("\x1bPq");
    output.push_str(&format!("\"1;1;{width};{height}"));
    for (index, used) in used_colors.iter().enumerate() {
        if *used {
            let color = sixel_palette_color(index as u16);
            output.push_str(&format!(
                "#{};2;{};{};{}",
                index,
                sixel_percent(color.r),
                sixel_percent(color.g),
                sixel_percent(color.b)
            ));
        }
    }

    for band_y in (0..height).step_by(6) {
        let mut wrote_band = false;
        for color in used_colors
            .iter()
            .enumerate()
            .filter_map(|(index, used)| used.then_some(index as u16))
        {
            let masks = sixel_band_masks(pixels, width, height, band_y, color);
            if masks.iter().all(|mask| *mask == 0) {
                continue;
            }
            if wrote_band {
                output.push('$');
            }
            output.push_str(&format!("#{color}"));
            output.push_str(&encode_sixel_mask_run(&masks));
            wrote_band = true;
        }
        if band_y + 6 < height {
            output.push('-');
        }
    }

    output.push_str("\x1b\\\n");
    output
}

/// 计算一条 sixel 6 像素高 band 内指定颜色的列掩码。
///
/// 参数:
/// - `pixels`: 量化像素
/// - `width`: 图片宽度
/// - `height`: 图片高度
/// - `band_y`: band 起始 Y
/// - `color`: 颜色索引
///
/// 返回:
/// - 每一列对应的 6-bit sixel 掩码
fn sixel_band_masks(
    pixels: &[Option<u16>],
    width: usize,
    height: usize,
    band_y: usize,
    color: u16,
) -> Vec<u8> {
    let mut masks = vec![0; width];
    for x in 0..width {
        let mut mask = 0;
        for bit in 0..6 {
            let y = band_y + bit;
            if y >= height {
                continue;
            }
            if pixels[y * width + x] == Some(color) {
                mask |= 1 << bit;
            }
        }
        masks[x] = mask;
    }
    masks
}

/// 使用 sixel 的重复编码压缩列掩码。
///
/// 参数:
/// - `masks`: 6-bit 列掩码
///
/// 返回:
/// - sixel 像素数据
fn encode_sixel_mask_run(masks: &[u8]) -> String {
    let end = masks
        .iter()
        .rposition(|mask| *mask != 0)
        .map(|index| index + 1)
        .unwrap_or(0);
    let mut output = String::new();
    let mut index = 0;
    while index < end {
        let mask = masks[index];
        let mut count = 1;
        while index + count < end && masks[index + count] == mask {
            count += 1;
        }
        let ch = char::from(63 + mask);
        if count >= 4 {
            output.push_str(&format!("!{count}{ch}"));
        } else {
            for _ in 0..count {
                output.push(ch);
            }
        }
        index += count;
    }
    output
}

/// 返回像素对应的 216 色 sixel 调色板索引。
///
/// 参数:
/// - `pixel`: 不透明 RGB 像素
///
/// 返回:
/// - 颜色索引
fn sixel_palette_index(pixel: Rgba) -> u16 {
    let r = quantize_sixel_channel(pixel.r);
    let g = quantize_sixel_channel(pixel.g);
    let b = quantize_sixel_channel(pixel.b);
    u16::from(r) * 36 + u16::from(g) * 6 + u16::from(b)
}

/// 将 0-255 色彩通道量化到 6 级。
///
/// 参数:
/// - `value`: 原通道值
///
/// 返回:
/// - 0..=5 的量化索引
fn quantize_sixel_channel(value: u8) -> u8 {
    ((u16::from(value) * 5 + 127) / 255) as u8
}

/// 返回 216 色 sixel 调色板中某个索引的 RGB 颜色。
///
/// 参数:
/// - `index`: 调色板索引
///
/// 返回:
/// - RGB 颜色
fn sixel_palette_color(index: u16) -> Rgba {
    let r = usize::from(index / 36);
    let g = usize::from((index / 6) % 6);
    let b = usize::from(index % 6);
    Rgba {
        r: SIXEL_COLOR_STEPS[r],
        g: SIXEL_COLOR_STEPS[g],
        b: SIXEL_COLOR_STEPS[b],
        a: 255,
    }
}

/// sixel 颜色定义使用 0-100 百分比。
///
/// 参数:
/// - `value`: 0-255 通道值
///
/// 返回:
/// - 0-100 百分比
fn sixel_percent(value: u8) -> u8 {
    ((u16::from(value) * 100 + 127) / 255) as u8
}

/// 计算 sixel 的目标像素尺寸。
///
/// 参数:
/// - `source_width`: 原图宽度
/// - `source_height`: 原图高度
///
/// 返回:
/// - 目标像素宽高
fn sixel_dimensions(
    source_width: usize,
    source_height: usize,
    size: &TerminalImageSize,
) -> (usize, usize) {
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let max_width = match size.width_cells {
        Some(value) => value
            .saturating_mul(SIXEL_CELL_WIDTH_PX)
            .clamp(SIXEL_CELL_WIDTH_PX, SIXEL_MAX_WIDTH_PX),
        None => usize::from(cols)
            .saturating_mul(SIXEL_CELL_WIDTH_PX)
            .clamp(240, SIXEL_MAX_WIDTH_PX),
    };
    let max_height = match size.height_cells {
        Some(value) => value
            .saturating_mul(SIXEL_CELL_HEIGHT_PX)
            .clamp(SIXEL_CELL_HEIGHT_PX, SIXEL_MAX_HEIGHT_PX),
        None => usize::from(rows)
            .saturating_mul(SIXEL_CELL_HEIGHT_PX)
            .clamp(160, SIXEL_MAX_HEIGHT_PX),
    };
    fit_dimensions(source_width, source_height, max_width, max_height)
}

/// 计算 ANSI fallback 的目标字符图尺寸。
///
/// 参数:
/// - `source_width`: 原图宽度
/// - `source_height`: 原图高度
///
/// 返回:
/// - 目标像素宽高
fn ansi_dimensions(
    source_width: usize,
    source_height: usize,
    size: &TerminalImageSize,
) -> (usize, usize) {
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let max_width = size
        .width_cells
        .unwrap_or_else(|| usize::from(cols.clamp(20, 120)))
        .clamp(1, 300);
    let max_rows = size
        .height_cells
        .unwrap_or_else(|| usize::from((rows / 2).clamp(8, 40)))
        .clamp(1, 200);
    let max_height = max_rows * 2;
    fit_dimensions(source_width, source_height, max_width, max_height)
}

/// 按最大宽高等比缩放图片尺寸。
///
/// 参数:
/// - `source_width`: 原图宽度
/// - `source_height`: 原图高度
/// - `max_width`: 最大输出宽度
/// - `max_height`: 最大输出高度
///
/// 返回:
/// - 输出宽高
fn fit_dimensions(
    source_width: usize,
    source_height: usize,
    max_width: usize,
    max_height: usize,
) -> (usize, usize) {
    if source_width == 0 || source_height == 0 {
        return (1, 2);
    }
    if source_width <= max_width && source_height <= max_height {
        return (source_width.max(1), source_height.max(1));
    }
    let source_ratio = source_width as f32 / source_height as f32;
    let frame_ratio = max_width as f32 / max_height as f32;
    let (width, height) = if source_ratio >= frame_ratio {
        let width = max_width;
        let height = ((width as f32 / source_ratio).round() as usize).clamp(1, max_height);
        (width, height)
    } else {
        let height = max_height;
        let width = ((height as f32 * source_ratio).round() as usize).clamp(1, max_width);
        (width, height)
    };
    (width.max(1), height.max(1))
}

/// 从缩放后的坐标采样原图像素。
///
/// 参数:
/// - `image`: 原图
/// - `x`: 目标 X
/// - `y`: 目标 Y
/// - `target_width`: 目标宽度
/// - `target_height`: 目标高度
///
/// 返回:
/// - 原图采样像素
fn sample_resized_pixel(
    image: &RasterImage,
    x: usize,
    y: usize,
    target_width: usize,
    target_height: usize,
) -> Rgba {
    if target_width >= image.width && target_height >= image.height {
        let source_x = (x * image.width / target_width).min(image.width.saturating_sub(1));
        let source_y = (y * image.height / target_height).min(image.height.saturating_sub(1));
        return image.pixels[source_y * image.width + source_x];
    }

    let start_x = x * image.width / target_width;
    let end_x = ((x + 1) * image.width).div_ceil(target_width);
    let start_y = y * image.height / target_height;
    let end_y = ((y + 1) * image.height).div_ceil(target_height);
    average_pixels(
        image,
        start_x.min(image.width.saturating_sub(1)),
        end_x.clamp(start_x + 1, image.width),
        start_y.min(image.height.saturating_sub(1)),
        end_y.clamp(start_y + 1, image.height),
    )
}

/// 对原图矩形区域做简单平均采样。
///
/// 参数:
/// - `image`: 原图
/// - `start_x`: 起始 X
/// - `end_x`: 结束 X
/// - `start_y`: 起始 Y
/// - `end_y`: 结束 Y
///
/// 返回:
/// - 平均像素
fn average_pixels(
    image: &RasterImage,
    start_x: usize,
    end_x: usize,
    start_y: usize,
    end_y: usize,
) -> Rgba {
    let mut total_r = 0u32;
    let mut total_g = 0u32;
    let mut total_b = 0u32;
    let mut total_a = 0u32;
    let mut count = 0u32;
    for y in start_y..end_y {
        for x in start_x..end_x {
            let pixel = image.pixels[y * image.width + x];
            total_r += u32::from(pixel.r);
            total_g += u32::from(pixel.g);
            total_b += u32::from(pixel.b);
            total_a += u32::from(pixel.a);
            count += 1;
        }
    }
    if count == 0 {
        return Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
    }
    Rgba {
        r: (total_r / count) as u8,
        g: (total_g / count) as u8,
        b: (total_b / count) as u8,
        a: (total_a / count) as u8,
    }
}

/// 渲染一个上/下半像素字符格。
///
/// 参数:
/// - `top`: 上半像素
/// - `bottom`: 下半像素
///
/// 返回:
/// - ANSI 文本片段
fn render_halfblock_cell(top: Rgba, bottom: Rgba) -> String {
    let top_visible = top.a >= ANSI_ALPHA_THRESHOLD;
    let bottom_visible = bottom.a >= ANSI_ALPHA_THRESHOLD;
    match (top_visible, bottom_visible) {
        (true, true) => {
            let top = blend_over_background(top);
            let bottom = blend_over_background(bottom);
            format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                top.r, top.g, top.b, bottom.r, bottom.g, bottom.b
            )
        }
        (true, false) => {
            let top = blend_over_background(top);
            format!("\x1b[49m\x1b[38;2;{};{};{}m▀", top.r, top.g, top.b)
        }
        (false, true) => {
            let bottom = blend_over_background(bottom);
            format!("\x1b[49m\x1b[38;2;{};{};{}m▄", bottom.r, bottom.g, bottom.b)
        }
        (false, false) => "\x1b[0m ".to_string(),
    }
}

/// 将半透明像素合成到深色背景，避免透明 PNG 在 ANSI fallback 下发灰。
///
/// 参数:
/// - `pixel`: 原像素
///
/// 返回:
/// - 合成后的不透明像素
fn blend_over_background(pixel: Rgba) -> Rgba {
    if pixel.a == 255 {
        return pixel;
    }
    let alpha = u16::from(pixel.a);
    let inverse = 255 - alpha;
    Rgba {
        r: blend_channel(pixel.r, ANSI_FALLBACK_BG.r, alpha, inverse),
        g: blend_channel(pixel.g, ANSI_FALLBACK_BG.g, alpha, inverse),
        b: blend_channel(pixel.b, ANSI_FALLBACK_BG.b, alpha, inverse),
        a: 255,
    }
}

/// 混合单个色彩通道。
///
/// 参数:
/// - `foreground`: 前景通道
/// - `background`: 背景通道
/// - `alpha`: 前景 alpha
/// - `inverse`: 背景 alpha
///
/// 返回:
/// - 混合后的通道
fn blend_channel(foreground: u8, background: u8, alpha: u16, inverse: u16) -> u8 {
    ((u16::from(foreground) * alpha + u16::from(background) * inverse + 127) / 255) as u8
}

/// 计算 chafa 图片显示尺寸。
///
/// 返回:
/// - chafa `--size` 参数
fn chafa_size(size: &TerminalImageSize) -> Option<String> {
    if size.width_cells.is_some() || size.height_cells.is_some() {
        let width = size
            .width_cells
            .map(|value| value.min(300).to_string())
            .unwrap_or_default();
        let height = size
            .height_cells
            .map(|value| value.min(200).to_string())
            .unwrap_or_default();
        return Some(format!("{width}x{height}"));
    }
    let (cols, rows) = terminal::size().ok()?;
    let width = cols.clamp(20, 120);
    let height = (rows / 2).clamp(8, 40);
    Some(format!("{width}x{height}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_iterm_terminal_program() {
        std::env::set_var("TERM_PROGRAM", "iTerm.app");
        assert!(supports_iterm_inline_image());
        std::env::remove_var("TERM_PROGRAM");
    }

    #[test]
    fn detects_windows_terminal_session() {
        std::env::set_var("WT_SESSION", "session-id");
        assert!(supports_windows_terminal_sixel());
        std::env::remove_var("WT_SESSION");
    }

    #[test]
    fn renders_png_with_ansi_halfblock_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("sample.png");
        write_test_rgba_png(
            &path,
            2,
            2,
            &[
                Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                Rgba {
                    r: 0,
                    g: 255,
                    b: 0,
                    a: 255,
                },
                Rgba {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                },
                Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            ],
        );
        let output = render_ansi_halfblock_image(&path, &TerminalImageSize::default()).unwrap();
        assert!(output.contains("\x1b[38;2;255;0;0m"));
        assert!(output.contains('▀') || output.contains('▄'));
    }

    #[test]
    fn renders_jpeg_with_ansi_halfblock_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("sample.jpg");
        let file = File::create(&path).unwrap();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 90);
        encoder
            .encode(
                &[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255],
                2,
                2,
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        let output = render_ansi_halfblock_image(&path, &TerminalImageSize::default()).unwrap();
        assert!(output.contains("\x1b[38;2;"));
        assert!(output.contains('▀') || output.contains('▄'));
    }

    #[test]
    fn renders_png_with_builtin_sixel_protocol() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("sample.png");
        write_test_rgba_png(
            &path,
            2,
            2,
            &[
                Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
                Rgba {
                    r: 0,
                    g: 255,
                    b: 0,
                    a: 255,
                },
                Rgba {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 255,
                },
                Rgba {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            ],
        );
        let output = render_sixel_image(&path, &TerminalImageSize::default()).unwrap();
        assert!(output.starts_with("\x1bPq"));
        assert!(output.contains("\"1;1;"));
        assert!(output.contains("#180;2;100;0;0"));
        assert!(output.ends_with("\x1b\\\n"));
    }

    #[test]
    fn sixel_omits_fully_transparent_pixels() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("transparent.png");
        write_test_rgba_png(
            &path,
            2,
            1,
            &[
                Rgba {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
                Rgba {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                },
            ],
        );
        let output = render_sixel_image(&path, &TerminalImageSize::default()).unwrap();
        assert!(!output.contains("#0;2;0;0;0"));
        assert!(output.contains("#180;2;100;0;0"));
    }

    fn write_test_rgba_png(path: &Path, width: u32, height: u32, pixels: &[Rgba]) {
        let file = File::create(path).unwrap();
        let writer = BufWriter::new(file);
        let mut encoder = png::Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        let bytes = pixels
            .iter()
            .flat_map(|pixel| [pixel.r, pixel.g, pixel.b, pixel.a])
            .collect::<Vec<_>>();
        writer.write_image_data(&bytes).unwrap();
    }
}
