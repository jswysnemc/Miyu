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
    fn wezterm_prefers_iterm_protocol_over_kitty() {
        std::env::set_var("TERM_PROGRAM", "WezTerm");
        std::env::remove_var("KITTY_WINDOW_ID");
        std::env::set_var("TERM", "xterm-256color");
        assert!(!supports_kitty_graphics());
        assert!(supports_iterm_inline_image());
        std::env::remove_var("TERM_PROGRAM");
        std::env::remove_var("TERM");
    }

    #[test]
    fn kitty_payload_includes_cell_size_and_reserves_rows() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("sample.png");
        // 16x32 像素 + 默认 8x16 单元格 => 至少 2 列 2 行
        let pixels = std::iter::repeat(Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        })
        .take(16 * 32)
        .collect::<Vec<_>>();
        write_test_rgba_png(&path, 16, 32, &pixels);
        let output = render_kitty_image(&path).unwrap();
        assert!(output.contains("\x1b_Gf=100,a=T,q=2,c="));
        assert!(output.contains(",r="));
        assert!(output.ends_with('\n'));
        let trailing_newlines = output.chars().rev().take_while(|ch| *ch == '\n').count();
        assert!(trailing_newlines >= 2);
    }

    #[test]
    fn encode_kitty_png_supports_explicit_cells_without_newlines() {
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
        let output = encode_kitty_png(&path, Some(8), Some(3)).unwrap();
        assert!(output.contains("f=100,a=T,q=2,c=8,r=3"));
        assert!(!output.ends_with('\n'));
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

    #[test]
    fn table_image_dimensions_width_first_keeps_aspect() {
        // 图 240x48，格 10x20：限制 12 列
        // base_r(12)=2，+1 余量 => 3
        let (cols, rows) = table_image_cell_dimensions(240, 48, 10, 20, 12, 16);
        assert_eq!(cols, 12);
        assert_eq!(rows, 3);
    }

    #[test]
    fn table_image_dimensions_natural_size_without_clamp() {
        // 图 240x48，格 10x20：自然 24 列
        // base_r(24)=3，+1 => 4
        let (cols, rows) = table_image_cell_dimensions(240, 48, 10, 20, 48, 16);
        assert_eq!(cols, 24);
        assert_eq!(rows, 4);
    }

    #[test]
    fn table_image_dimensions_height_cap_recomputes_cols() {
        // 100x400，格 10x20，max_rows=4；收 1 列后行高仍不超过上限
        let (cols, rows) = table_image_cell_dimensions(100, 400, 10, 20, 20, 4);
        assert!(rows <= 4);
        assert!(cols >= 1 && cols <= 20);
    }

    #[test]
    fn table_image_dimensions_clamps_abnormal_cell_height() {
        // ioctl 给过大格高 40（宽 10 → 比 4:1）：应钳到 2:1 的 20，行数变多
        let (cols_bad, rows_bad) = table_image_cell_dimensions(240, 48, 10, 40, 48, 16);
        let (cols_ok, rows_ok) = table_image_cell_dimensions(240, 48, 10, 20, 48, 16);
        assert_eq!(cols_bad, cols_ok);
        assert_eq!(rows_bad, rows_ok);
        assert!(rows_ok >= 3);
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
