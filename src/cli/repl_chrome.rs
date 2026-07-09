use super::repl_text::visible_width;
use super::*;
use crate::config::AppConfig;
use crate::state::StateStore;

/// REPL 底栏与输入框 chrome 状态。
#[derive(Debug, Clone)]
pub(super) struct ReplChrome {
    pub(super) mode: AgentMode,
    pub(super) model: String,
    pub(super) thinking: String,
    pub(super) context_ratio: f32,
    pub(super) context_window_tokens: usize,
}

impl ReplChrome {
    /// 从当前配置与会话状态构造 chrome。
    ///
    /// 参数:
    /// - `config`: 应用配置
    /// - `state`: 会话状态
    /// - `mode`: 当前 Agent 模式
    ///
    /// 返回:
    /// - chrome 状态
    pub(super) fn from_runtime(
        config: &AppConfig,
        state: &StateStore,
        mode: AgentMode,
    ) -> Self {
        let provider = config.provider(None).ok();
        let model = provider
            .map(|item| item.default_model.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "-".to_string());
        let thinking = provider
            .map(|item| item.thinking_level.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "auto".to_string());
        let context_limit = config.active_context_chars().unwrap_or(128_000);
        let snapshot = state.session_snapshot(context_limit).ok();
        Self {
            mode,
            model,
            thinking,
            context_ratio: snapshot
                .as_ref()
                .map(|item| item.context_token_ratio)
                .unwrap_or(0.0),
            context_window_tokens: snapshot
                .as_ref()
                .map(|item| item.context_window_tokens)
                .unwrap_or(context_limit),
        }
    }

    /// 更新模式（Tab 切换时）。
    ///
    /// 参数:
    /// - `mode`: 新模式
    pub(super) fn set_mode(&mut self, mode: AgentMode) {
        self.mode = mode;
    }

    /// 左侧上下文占用文案。
    ///
    /// 返回:
    /// - 如 `0.0%/272k (auto)`
    pub(super) fn context_status(&self) -> String {
        let pct = (self.context_ratio * 100.0).clamp(0.0, 999.9);
        format!(
            "{pct:.1}%/{} (auto)",
            format_token_k(self.context_window_tokens)
        )
    }

    /// 右侧模型与思考等级文案。
    ///
    /// 返回:
    /// - 如 `gpt-4o · xhigh`
    pub(super) fn model_status(&self) -> String {
        format!("{} · {}", self.model, self.thinking)
    }

    /// 模式纯文本（用于宽度计算）。
    ///
    /// 返回:
    /// - `yolo` / `plan`
    pub(super) fn mode_plain(&self) -> &'static str {
        match self.mode {
            AgentMode::Yolo => "yolo",
            AgentMode::Plan => "plan",
        }
    }

    /// 模式标签（小写极简，带颜色）。
    ///
    /// 返回:
    /// - 带颜色的 `yolo` / `plan`
    pub(super) fn mode_status(&self) -> String {
        match self.mode {
            AgentMode::Yolo => "\x1b[38;5;208myolo\x1b[0m".to_string(),
            AgentMode::Plan => "\x1b[36mplan\x1b[0m".to_string(),
        }
    }

    /// 底栏整行：`yolo  0.0%/272k (auto) … model · thinking`。
    ///
    /// 参数:
    /// - `cols`: 终端列数
    ///
    /// 返回:
    /// - 已着色状态行
    pub(super) fn footer_line(&self, cols: usize) -> String {
        let cols = cols.max(1);
        let context = self.context_status();
        let right = self.model_status();
        let left_w = visible_width(self.mode_plain()) + 2 + visible_width(&context);
        let right_w = visible_width(&right);
        if left_w + right_w + 1 >= cols {
            return format!(
                "{}  \x1b[2m{} {}\x1b[0m",
                self.mode_status(),
                context,
                right
            );
        }
        let gap = cols.saturating_sub(left_w + right_w);
        format!(
            "{}  \x1b[2m{}{}{}\x1b[0m",
            self.mode_status(),
            context,
            " ".repeat(gap),
            right
        )
    }
}

/// 将 token 数格式化为 `272k` 风格。
///
/// 参数:
/// - `value`: token 数
///
/// 返回:
/// - 缩写文本
fn format_token_k(value: usize) -> String {
    if value >= 1_000 {
        let scaled = value as f64 / 1_000.0;
        if scaled >= 10.0 {
            format!("{scaled:.0}k")
        } else {
            format!("{scaled:.1}k")
        }
    } else {
        value.to_string()
    }
}

/// 生成全宽浅色分隔线。
///
/// 参数:
/// - `cols`: 终端列数
///
/// 返回:
/// - 带样式的分隔线
/// 输入框上下分隔线样式。
///
/// 使用柔和蓝色，区别于正文里的 dim 水平线（`\x1b[2m`）与代码块青色边框（`\x1b[36m`）。
const CHROME_RULE_STYLE: &str = "\x1b[38;5;67m";

/// 生成输入框顶/底分隔线。
///
/// 参数:
/// - `cols`: 终端列数
///
/// 返回:
/// - 带颜色的整行分隔线
pub(super) fn chrome_rule(cols: usize) -> String {
    format!("{CHROME_RULE_STYLE}{}\x1b[0m", "─".repeat(cols.max(1)))
}

/// 生成左右对齐的状态行。
///
/// 参数:
/// - `left`: 左侧文本（无样式）
/// - `right`: 右侧文本（无样式）
/// - `cols`: 终端列数
///
/// 返回:
/// - 带 dim 样式的状态行
pub(super) fn chrome_status_line(left: &str, right: &str, cols: usize) -> String {
    let cols = cols.max(1);
    let left_w = visible_width(left);
    let right_w = visible_width(right);
    if left_w + right_w + 1 >= cols {
        return format!("\x1b[2m{left} {right}\x1b[0m");
    }
    let gap = cols.saturating_sub(left_w + right_w);
    format!("\x1b[2m{left}{}{right}\x1b[0m", " ".repeat(gap))
}

/// 极简 chrome 固定占用行数：顶线 + 底线 + 状态（模式并入状态行左侧）。
///
/// 返回:
/// - 固定行数
pub(super) fn chrome_fixed_rows() -> u16 {
    3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_line_keeps_left_and_right() {
        let line = chrome_status_line("0.0%/272k (auto)", "gpt · xhigh", 40);
        assert!(line.contains("0.0%/272k (auto)"));
        assert!(line.contains("gpt · xhigh"));
    }

    #[test]
    fn footer_puts_mode_before_context() {
        let chrome = ReplChrome {
            mode: AgentMode::Yolo,
            model: "gpt".into(),
            thinking: "xhigh".into(),
            context_ratio: 0.0,
            context_window_tokens: 272_000,
        };
        let line = chrome.footer_line(48);
        let plain = strip_ansi(&line);
        assert!(plain.starts_with("yolo"));
        assert!(plain.contains("0.0%/272k (auto)"));
        assert!(plain.contains("gpt · xhigh"));
    }

    fn strip_ansi(text: &str) -> String {
        let mut out = String::new();
        let mut escape = false;
        for ch in text.chars() {
            if ch == '\x1b' {
                escape = true;
                continue;
            }
            if escape {
                if ch == 'm' {
                    escape = false;
                }
                continue;
            }
            out.push(ch);
        }
        out
    }

    #[test]
    fn format_token_k_scales_thousands() {
        assert_eq!(format_token_k(272_000), "272k");
        assert_eq!(format_token_k(1_500), "1.5k");
        assert_eq!(format_token_k(42), "42");
    }

    #[test]
    fn chrome_rule_uses_distinct_color_not_plain_dim() {
        let line = chrome_rule(8);
        assert!(line.contains(CHROME_RULE_STYLE));
        assert!(!line.starts_with("\x1b[2m"));
        assert_eq!(strip_ansi(&line), "────────");
    }
}
