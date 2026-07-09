use unicode_width::UnicodeWidthChar;

/// 已按指定宽度预换行的 ANSI 终端行。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AnsiLine {
    text: String,
}

impl AnsiLine {
    /// 创建一条预换行 ANSI 终端行。
    ///
    /// 参数:
    /// - `text`: 不包含换行符的 ANSI 文本
    ///
    /// 返回:
    /// - ANSI 终端行
    pub(crate) fn new(text: String) -> Self {
        Self { text }
    }

    /// 返回可直接写入终端的 ANSI 文本。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 不包含换行符的 ANSI 文本
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }

    /// 将 ANSI 文本块拆分并预换行到指定终端宽度。
    ///
    /// 参数:
    /// - `text`: 原始 ANSI 文本块
    /// - `width`: 当前终端列数
    ///
    /// 返回:
    /// - 预换行后的终端行
    pub(crate) fn wrap_block(text: &str, width: usize) -> Vec<Self> {
        let mut lines = Vec::new();
        for raw_line in text.split('\n') {
            lines.extend(wrap_line(raw_line, width));
        }
        lines
    }
}

/// 按显示宽度切分单行 ANSI 文本，并在续行恢复活动样式。
fn wrap_line(text: &str, width: usize) -> Vec<AnsiLine> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut active_sgr = String::new();
    let fill_to_end = text.contains("\x1b[K");
    let mut index = 0usize;

    while index < text.len() {
        let ch = text[index..].chars().next().unwrap_or_default();
        if ch == '\x1b' {
            let end = crate::render::terminal_image::escape_sequence_end(text, index);
            let sequence = &text[index..end];
            match sequence.chars().last() {
                Some('m') => {
                    update_active_sgr(&mut active_sgr, sequence);
                    current.push_str(sequence);
                }
                Some('K') => {}
                _ => current.push_str(sequence),
            }
            index = end.max(index + ch.len_utf8());
            continue;
        }

        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width > 0 && current_width.saturating_add(char_width) > width {
            lines.push(finish_line(&current, fill_to_end));
            current = active_sgr.clone();
            current_width = 0;
        }
        current.push(ch);
        current_width = current_width.saturating_add(char_width);
        index += ch.len_utf8();
    }

    if !current.is_empty() || lines.is_empty() {
        lines.push(finish_line(&current, fill_to_end));
    }
    lines
}

/// 更新续行需要恢复的 SGR 样式序列。
fn update_active_sgr(active_sgr: &mut String, sequence: &str) {
    let Some(body) = sequence.strip_prefix("\x1b[") else {
        return;
    };
    let params = body.strip_suffix('m').unwrap_or(body);
    let reset = params.is_empty() || params.split(';').any(|value| value == "0");
    if reset {
        active_sgr.clear();
    }
    if sequence != "\x1b[m" && sequence != "\x1b[0m" {
        active_sgr.push_str(sequence);
    }
}

/// 结束预换行行并保持 diff 的 EL 背景填充语义。
fn finish_line(text: &str, fill_to_end: bool) -> AnsiLine {
    let mut output = text.to_string();
    if fill_to_end {
        output.push_str("\x1b[K");
    }
    output.push_str("\x1b[0m");
    AnsiLine::new(output)
}
