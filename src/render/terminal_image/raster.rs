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
