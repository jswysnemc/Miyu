use anyhow::Result;
use crossterm::cursor::MoveToColumn;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DOT_COUNT: usize = 8;
const TRAIL_LEN: usize = 6;
const HOLD_END: usize = 9;
const HOLD_START: usize = 30;
const INTERVAL: Duration = Duration::from_millis(40);
const ACCENT_RGB: Rgb = Rgb(133, 153, 0);
const BASE_RGB: (f64, f64, f64) = (0.522, 0.600, 0.0);
const ACTIVE_DOT: &str = "■";
const INACTIVE_DOT: &str = "⬝";
const DEFAULT_FACE: &str = "(◕‿◕)";
const BLINK_FACE: &str = "(-‿-)";
const SLEEPY_FACE: &str = "(◡‿◡)";
const GLANCE_FACES: [&str; 3] = ["(◐‿◐)", "(◑‿◑)", "(◔‿◔)"];

pub(crate) struct WaitSpinner {
    state: Arc<Mutex<WaitSpinnerState>>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

struct WaitSpinnerState {
    phase: String,
    start: Instant,
    seed: u64,
}

#[derive(Clone, Copy)]
struct ScannerState {
    active_pos: usize,
    is_holding: bool,
    hold_progress: usize,
    hold_total: usize,
    move_progress: usize,
    move_total: usize,
    forward: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rgb(u8, u8, u8);

impl WaitSpinner {
    /// 判断当前终端是否适合显示等待动画。
    ///
    /// 返回:
    /// - 是否可以显示等待动画
    pub(crate) fn supported() -> bool {
        io::stdout().is_terminal()
    }

    /// 启动等待响应动画。
    ///
    /// 参数:
    /// - `phase`: 初始状态文本
    ///
    /// 返回:
    /// - 等待动画控制器
    pub(crate) fn start(phase: String) -> Self {
        let state = Arc::new(Mutex::new(WaitSpinnerState {
            phase,
            start: Instant::now(),
            seed: spinner_seed(),
        }));
        let running = Arc::new(AtomicBool::new(true));
        let thread_state = Arc::clone(&state);
        let thread_running = Arc::clone(&running);
        let handle = thread::spawn(move || run_spinner_loop(thread_state, thread_running));
        Self {
            state,
            running,
            handle: Some(handle),
        }
    }

    /// 更新等待动画的状态文本。
    ///
    /// 参数:
    /// - `phase`: 新状态文本
    pub(crate) fn set_phase(&self, phase: String) {
        if let Ok(mut state) = self.state.lock() {
            state.phase = phase;
        }
    }

    /// 停止等待动画并清理当前行。
    ///
    /// 返回:
    /// - 停止是否成功
    pub(crate) fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        clear_spinner_line()
    }
}

impl Drop for WaitSpinner {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// 执行等待动画循环。
///
/// 参数:
/// - `state`: 等待动画共享状态
/// - `running`: 是否继续运行动画
fn run_spinner_loop(state: Arc<Mutex<WaitSpinnerState>>, running: Arc<AtomicBool>) {
    let mut frame = 0usize;
    let mut cycle = 0usize;
    while running.load(Ordering::SeqCst) {
        let line = match state.lock() {
            Ok(state) => render_frame(frame, cycle, &state),
            Err(_) => String::new(),
        };
        if !line.is_empty() {
            let _ = write_spinner_line(&line);
        }
        thread::sleep(INTERVAL);
        frame += 1;
        if frame >= total_frames() {
            frame = 0;
            cycle += 1;
        }
    }
}

/// 渲染单帧等待动画。
///
/// 参数:
/// - `frame`: 当前周期内帧序号
/// - `cycle`: 已完成周期数量
/// - `state`: 等待动画共享状态
///
/// 返回:
/// - 单行 ANSI 文本
fn render_frame(frame: usize, cycle: usize, state: &WaitSpinnerState) -> String {
    let scanner = scanner_state(frame);
    let inactive_color = inactive_rgb(fade_factor(scanner));
    let abs_frame = cycle * total_frames() + frame;
    let mut output = String::new();
    output.push_str(&paint_rgb(
        face_for_frame(abs_frame, state.seed),
        ACCENT_RGB,
    ));
    output.push(' ');
    for index in 0..DOT_COUNT {
        let dot = color_index(index, scanner)
            .filter(|color_index| *color_index < TRAIL_LEN)
            .map(|color_index| paint_rgb(ACTIVE_DOT, trail_rgb(color_index)))
            .unwrap_or_else(|| paint_rgb(INACTIVE_DOT, inactive_color));
        output.push_str(&dot);
    }
    output.push(' ');
    output.push_str(&paint_bold_rgb(&state.phase, ACCENT_RGB));
    let elapsed = state.start.elapsed();
    if elapsed > Duration::from_secs(1) {
        output.push_str(&paint_faint(&format!(" {:.1}s", elapsed.as_secs_f64())));
    }
    output
}

/// 写入等待动画行。
///
/// 参数:
/// - `line`: 已渲染的等待动画文本
///
/// 返回:
/// - 写入是否成功
fn write_spinner_line(line: &str) -> Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
    write!(stdout, "{line}")?;
    stdout.flush()?;
    Ok(())
}

/// 清理等待动画行。
///
/// 返回:
/// - 清理是否成功
fn clear_spinner_line() -> Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
    stdout.flush()?;
    Ok(())
}

/// 计算完整往返周期帧数。
///
/// 返回:
/// - 完整周期帧数
fn total_frames() -> usize {
    DOT_COUNT + HOLD_END + (DOT_COUNT - 1) + HOLD_START
}

/// 计算扫描点状态。
///
/// 参数:
/// - `frame`: 当前周期内帧序号
///
/// 返回:
/// - 扫描点状态
fn scanner_state(mut frame: usize) -> ScannerState {
    let forward_frames = DOT_COUNT;
    let backward_frames = DOT_COUNT - 1;
    if frame < forward_frames {
        return ScannerState {
            active_pos: frame,
            is_holding: false,
            hold_progress: 0,
            hold_total: 0,
            move_progress: frame,
            move_total: forward_frames,
            forward: true,
        };
    }
    frame -= forward_frames;
    if frame < HOLD_END {
        return ScannerState {
            active_pos: DOT_COUNT - 1,
            is_holding: true,
            hold_progress: frame,
            hold_total: HOLD_END,
            move_progress: 0,
            move_total: 0,
            forward: true,
        };
    }
    frame -= HOLD_END;
    if frame < backward_frames {
        return ScannerState {
            active_pos: DOT_COUNT - 2 - frame,
            is_holding: false,
            hold_progress: 0,
            hold_total: 0,
            move_progress: frame,
            move_total: backward_frames,
            forward: false,
        };
    }
    frame -= backward_frames;
    ScannerState {
        active_pos: 0,
        is_holding: true,
        hold_progress: frame,
        hold_total: HOLD_START,
        move_progress: 0,
        move_total: 0,
        forward: false,
    }
}

/// 计算指定位置的尾迹颜色索引。
///
/// 参数:
/// - `char_index`: 点位序号
/// - `state`: 当前扫描状态
///
/// 返回:
/// - 有颜色时返回尾迹索引，否则返回空
fn color_index(char_index: usize, state: ScannerState) -> Option<usize> {
    let distance = if state.forward {
        state.active_pos as isize - char_index as isize
    } else {
        char_index as isize - state.active_pos as isize
    };
    if state.is_holding {
        return usize::try_from(distance)
            .ok()
            .map(|distance| distance + state.hold_progress);
    }
    if distance == 0 {
        return Some(0);
    }
    if distance > 0 && distance < TRAIL_LEN as isize {
        return usize::try_from(distance).ok();
    }
    None
}

/// 计算非活动点的淡入淡出系数。
///
/// 参数:
/// - `state`: 当前扫描状态
///
/// 返回:
/// - 颜色系数
fn fade_factor(state: ScannerState) -> f64 {
    const MIN_ALPHA: f64 = 0.3;
    if state.is_holding && state.hold_total > 0 {
        let progress = (state.hold_progress as f64 / state.hold_total as f64).min(1.0);
        (1.0 - progress * (1.0 - MIN_ALPHA)).max(MIN_ALPHA)
    } else if !state.is_holding && state.move_total > 0 {
        let denom = state.move_total.saturating_sub(1).max(1);
        let progress = (state.move_progress as f64 / denom as f64).min(1.0);
        MIN_ALPHA + progress * (1.0 - MIN_ALPHA)
    } else {
        1.0
    }
}

/// 根据帧序号选择表情。
///
/// 参数:
/// - `frame`: 全局帧序号
/// - `seed`: 进程级随机种子
///
/// 返回:
/// - 表情文本
fn face_for_frame(frame: usize, seed: u64) -> &'static str {
    let mut blink_pos = 0usize;
    let mut blink_segment = 0usize;
    while blink_pos <= frame {
        let hash = face_hash(blink_segment * 7 + seed as usize);
        let gap = 25 + hash % 25;
        if frame >= blink_pos + gap && frame < blink_pos + gap + 3 {
            return BLINK_FACE;
        }
        blink_pos += gap + 3;
        blink_segment += 1;
    }
    if frame > 200 {
        return SLEEPY_FACE;
    }
    let mut position = 0usize;
    let mut segment = 0usize;
    while position <= frame {
        let hash = face_hash(segment + seed as usize);
        let segment_len = 25 + hash % 50;
        if frame < position + segment_len {
            let index = (hash / 50) % (GLANCE_FACES.len() + 1);
            if index < GLANCE_FACES.len() {
                return GLANCE_FACES[index];
            }
            return DEFAULT_FACE;
        }
        position += segment_len;
        segment += 1;
    }
    DEFAULT_FACE
}

/// 生成表情选择用哈希值。
///
/// 参数:
/// - `value`: 输入数字
///
/// 返回:
/// - 哈希值
fn face_hash(value: usize) -> usize {
    let mut value = value as u64;
    value = ((value >> 16) ^ value).wrapping_mul(0x45d9f3b);
    value = ((value >> 16) ^ value).wrapping_mul(0x45d9f3b);
    ((value >> 16) ^ value) as usize
}

/// 生成当前进程的动画种子。
///
/// 返回:
/// - 动画种子
fn spinner_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64 & 0x7fff_ffff)
        .unwrap_or(0)
}

/// 计算尾迹颜色。
///
/// 参数:
/// - `index`: 尾迹索引
///
/// 返回:
/// - RGB 颜色
fn trail_rgb(index: usize) -> Rgb {
    let (mut r, mut g, mut b) = BASE_RGB;
    let alpha = match index {
        0 => 1.0,
        1 => {
            r = (r * 1.15).min(1.0);
            g = (g * 1.15).min(1.0);
            b = (b * 1.15).min(1.0);
            0.9
        }
        _ => 0.65_f64.powi(index.saturating_sub(1) as i32),
    };
    Rgb(
        clamp8(r * alpha * 255.0),
        clamp8(g * alpha * 255.0),
        clamp8(b * alpha * 255.0),
    )
}

/// 计算非活动点颜色。
///
/// 参数:
/// - `factor`: 淡入淡出系数
///
/// 返回:
/// - RGB 颜色
fn inactive_rgb(factor: f64) -> Rgb {
    let (r, g, b) = BASE_RGB;
    let alpha = 0.6 * factor;
    Rgb(
        clamp8(r * alpha * 255.0),
        clamp8(g * alpha * 255.0),
        clamp8(b * alpha * 255.0),
    )
}

/// 限制颜色通道范围。
///
/// 参数:
/// - `value`: 原始浮点颜色值
///
/// 返回:
/// - 8 位颜色通道
fn clamp8(value: f64) -> u8 {
    value.clamp(0.0, 255.0) as u8
}

/// 渲染真彩前景色文本。
///
/// 参数:
/// - `text`: 原始文本
/// - `rgb`: 文本颜色
///
/// 返回:
/// - 带 ANSI 样式的文本
fn paint_rgb(text: &str, rgb: Rgb) -> String {
    format!("\x1b[38;2;{};{};{}m{text}\x1b[0m", rgb.0, rgb.1, rgb.2)
}

/// 渲染加粗真彩前景色文本。
///
/// 参数:
/// - `text`: 原始文本
/// - `rgb`: 文本颜色
///
/// 返回:
/// - 带 ANSI 样式的文本
fn paint_bold_rgb(text: &str, rgb: Rgb) -> String {
    format!(
        "\x1b[1m\x1b[38;2;{};{};{}m{text}\x1b[0m",
        rgb.0, rgb.1, rgb.2
    )
}

/// 渲染弱化文本。
///
/// 参数:
/// - `text`: 原始文本
///
/// 返回:
/// - 带 ANSI 样式的文本
fn paint_faint(text: &str) -> String {
    format!("\x1b[2m{text}\x1b[0m")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_cycle_matches_expected_length() {
        assert_eq!(total_frames(), 54);
    }

    #[test]
    fn scanner_moves_forward_then_holds() {
        let moving = scanner_state(3);
        assert_eq!(moving.active_pos, 3);
        assert!(moving.forward);
        assert!(!moving.is_holding);
        let holding = scanner_state(DOT_COUNT);
        assert_eq!(holding.active_pos, DOT_COUNT - 1);
        assert!(holding.is_holding);
    }

    #[test]
    fn color_index_tracks_tail_direction() {
        let state = scanner_state(4);
        assert_eq!(color_index(4, state), Some(0));
        assert_eq!(color_index(3, state), Some(1));
        assert_eq!(color_index(7, state), None);
    }

    #[test]
    fn trail_colors_fade_after_head() {
        assert_eq!(trail_rgb(0), ACCENT_RGB);
        let tail = trail_rgb(4);
        assert!(tail.0 < ACCENT_RGB.0);
        assert!(tail.1 < ACCENT_RGB.1);
    }
}
