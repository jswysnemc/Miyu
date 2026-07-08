use crate::i18n::text as t;
use crate::state::failure_recovery::summary::{format_recovery_snapshot, has_visible_recovery};
use crate::state::SessionSnapshot;
use anyhow::Result;
use unicode_width::UnicodeWidthStr;

const LABEL_WIDTH: usize = 16;
const MIN_CARD_WIDTH: usize = 58;

/// 打印命令模式会话结束摘要。
///
/// 参数:
/// - `snapshot`: 当前会话状态快照
///
/// 返回:
/// - 打印是否成功
pub fn print_session_summary(snapshot: &SessionSnapshot) -> Result<()> {
    println!("{}", render_session_summary(snapshot));
    Ok(())
}

/// 渲染命令模式会话结束摘要。
///
/// 参数:
/// - `snapshot`: 当前会话状态快照
///
/// 返回:
/// - 可打印的摘要卡片
pub fn render_session_summary(snapshot: &SessionSnapshot) -> String {
    let rows = summary_rows(snapshot);
    let content_width = rows
        .iter()
        .map(|(label, value)| visible_width(&row_content(label, value)))
        .max()
        .unwrap_or_default()
        .max(MIN_CARD_WIDTH);
    let title = format!(" {} ", t("Miyu session", "Miyu 会话"));
    let mut output = String::new();
    output.push_str(&top_border(&title, content_width));
    for (label, value) in rows {
        output.push_str(&format_row(&label, &value, content_width));
    }
    output.push_str(&bottom_border(content_width));
    output
}

/// 构造摘要行。
///
/// 参数:
/// - `snapshot`: 当前会话状态快照
///
/// 返回:
/// - 摘要行列表
fn summary_rows(snapshot: &SessionSnapshot) -> Vec<(String, String)> {
    let context_percent = snapshot.context_ratio * 100.0;
    let last_usage = snapshot
        .usage
        .last_usage
        .as_ref()
        .map(|usage| {
            format_last_usage(
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.total_tokens,
            )
        })
        .unwrap_or_else(|| t("none", "无").to_string());
    let compaction = snapshot
        .compaction
        .as_ref()
        .map(|summary| {
            format!(
                "{} {}, {} {}",
                format_number(summary.compacted_turns as u64),
                t("turns", "轮"),
                t("updated", "更新于"),
                summary.updated_at
            )
        })
        .unwrap_or_else(|| t("none", "无").to_string());
    let checkpoint = if snapshot.checkpoint_count == 0 {
        t("none", "无").to_string()
    } else {
        let latest = snapshot
            .latest_checkpoint_at
            .as_deref()
            .unwrap_or_else(|| t("unknown", "未知"));
        format!(
            "{} {}, {} {} {}, {} {} {}, {} {}",
            format_number(snapshot.checkpoint_count as u64),
            t("checkpoints", "个检查点"),
            t("covered", "覆盖"),
            format_number(snapshot.checkpoint_covered_turns as u64),
            t("turns", "轮"),
            t("tail", "尾部"),
            format_number(snapshot.tail_turns as u64),
            t("turns", "轮"),
            t("latest", "最近"),
            latest
        )
    };

    let mut rows = vec![
        (
            t("Session ID", "会话 ID").to_string(),
            snapshot.session_id.clone(),
        ),
        (
            t("Turns", "轮次数").to_string(),
            format_number(snapshot.turn_count as u64),
        ),
        (
            t("Context", "上下文").to_string(),
            format!(
                "{} / {} {} ({context_percent:.1}%)",
                format_number(snapshot.context_chars as u64),
                format_number(snapshot.context_limit_chars as u64),
                t("chars", "字符")
            ),
        ),
        (
            t("Usage total", "累计用量").to_string(),
            format_total_usage(
                snapshot.usage.requests,
                snapshot.usage.prompt_tokens,
                snapshot.usage.completion_tokens,
                snapshot.usage.total_tokens,
            ),
        ),
        (t("Last usage", "最近用量").to_string(), last_usage),
        (t("Compaction", "压缩").to_string(), compaction),
        (t("Checkpoint", "检查点").to_string(), checkpoint),
    ];
    if let Some(epoch) = &snapshot.context_epoch {
        rows.push((
            t("Context Epoch", "上下文纪元").to_string(),
            format_context_epoch(epoch),
        ));
    }
    if let Some(memory) = &snapshot.session_memory {
        rows.push((
            t("Session Memory", "会话记忆").to_string(),
            format_session_memory(memory),
        ));
    }
    if snapshot.tool_history.call_count > 0 {
        rows.push((
            t("Tool History", "工具历史").to_string(),
            format_tool_history(&snapshot.tool_history),
        ));
    }
    if crate::runtime_recovery::has_visible_runtime_recovery(&snapshot.runtime_recovery) {
        rows.push((
            t("Runtime Recovery", "运行时恢复").to_string(),
            format_runtime_recovery(&snapshot.runtime_recovery),
        ));
    }
    if !snapshot.dynamic_sources.is_empty() {
        rows.push((
            t("Dynamic context", "动态上下文").to_string(),
            format_dynamic_sources(&snapshot.dynamic_sources),
        ));
    }
    if !snapshot.projection_warnings.is_empty() {
        rows.push((
            t("Projection", "投影").to_string(),
            format_projection_warnings(&snapshot.projection_warnings),
        ));
    }
    if let Some(active_run) = &snapshot.active_run {
        rows.push((
            t("Active run", "运行状态").to_string(),
            format_active_run(active_run),
        ));
    }
    if has_visible_recovery(&snapshot.recovery) {
        rows.push((
            t("Recovery", "恢复").to_string(),
            format_recovery_snapshot(&snapshot.recovery),
        ));
    }
    rows
}

/// 格式化投影警告。
///
/// 参数:
/// - `warnings`: 投影警告列表
///
/// 返回:
/// - 可显示的投影警告文本
fn format_projection_warnings(warnings: &[String]) -> String {
    warnings.join("; ")
}

/// 格式化 Runtime Recovery 摘要。
///
/// 参数:
/// - `summary`: Runtime Recovery 摘要
///
/// 返回:
/// - 可显示的 Runtime Recovery 文本
fn format_runtime_recovery(summary: &crate::runtime_recovery::RuntimeRecoverySummary) -> String {
    let mut parts = vec![
        format!(
            "{} {}",
            t("active", "活跃进程"),
            format_number(summary.active_process_count as u64)
        ),
        format!(
            "{} {}",
            t("stale", "异常进程"),
            format_number(summary.stale_process_count as u64)
        ),
    ];
    if let Some(failure) = &summary.latest_failure {
        let process = failure
            .process_id
            .as_deref()
            .unwrap_or_else(|| t("none", "无"));
        let mut failure_text = format!(
            "{} {}/{}, {} {}, {} {}",
            t("latest failure", "最近失败"),
            failure.kind.as_str(),
            failure.status.as_str(),
            t("process", "进程"),
            process,
            t("reason", "原因"),
            failure.reason
        );
        if let Some(last_safe_seq) = failure.last_safe_seq {
            failure_text.push_str(&format!(
                ", {} {}",
                t("last safe seq", "最后安全序号"),
                last_safe_seq
            ));
        }
        failure_text.push_str(&format!(
            ", {} {}",
            t("created", "记录于"),
            failure.created_at
        ));
        parts.push(failure_text);
    }
    parts.join(", ")
}

/// 格式化 active run 摘要。
///
/// 参数:
/// - `active_run`: active run 摘要
///
/// 返回:
/// - 可显示的 active run 文本
fn format_active_run(active_run: &crate::state::ActiveRunSummary) -> String {
    let lock_path = if active_run.lock_path.is_empty() {
        t("none", "无").to_string()
    } else {
        active_run.lock_path.clone()
    };
    format!(
        "{} {}, {} {}, {} {}, {} {}",
        t("owner", "owner"),
        active_run.owner,
        t("pid", "pid"),
        active_run.pid,
        t("started", "开始于"),
        active_run.started_at,
        t("lock", "锁文件"),
        lock_path
    )
}

/// 格式化会话工作记忆摘要。
///
/// 参数:
/// - `memory`: 会话工作记忆摘要
///
/// 返回:
/// - 可显示的会话工作记忆文本
fn format_session_memory(memory: &crate::state::SessionMemorySummary) -> String {
    let turn = memory
        .last_summarized_turn_id
        .as_deref()
        .unwrap_or_else(|| t("none", "无"));
    let checkpoint = memory
        .checkpoint_id
        .as_deref()
        .unwrap_or_else(|| t("none", "无"));
    let status = if memory.is_disabled {
        t("disabled", "已熔断")
    } else if memory.consecutive_failures > 0 {
        t("degraded", "降级")
    } else {
        t("active", "可用")
    };
    let mut output = format!(
        "{} {}, {} {}, {} {} {}, {} {}, {} {}, {} {}",
        t("seq", "序号"),
        memory.last_summarized_seq,
        t("turn", "轮次"),
        turn,
        t("sources", "来源"),
        format_number(memory.source_turn_count as u64),
        t("turns", "轮"),
        t("tokens", "Token"),
        format_number(memory.token_estimate as u64),
        t("checkpoint", "检查点"),
        checkpoint,
        t("status", "状态"),
        status
    );
    if memory.consecutive_failures > 0 {
        output.push_str(&format!(
            ", {} {}",
            t("failures", "失败次数"),
            format_number(memory.consecutive_failures as u64)
        ));
    }
    if let Some(error) = &memory.last_error {
        output.push_str(&format!(", {} {}", t("last error", "最近错误"), error));
    }
    output.push_str(&format!(
        ", {} {}",
        t("updated", "更新于"),
        memory.updated_at
    ));
    output
}

/// 格式化工具历史摘要。
///
/// 参数:
/// - `summary`: 工具历史摘要
///
/// 返回:
/// - 可显示的工具历史文本
fn format_tool_history(summary: &crate::state::ToolHistorySummary) -> String {
    let latest_tool = summary
        .latest_tool_name
        .as_deref()
        .unwrap_or_else(|| t("none", "无"));
    let latest_status = summary
        .latest_status
        .as_ref()
        .map(format_tool_status)
        .unwrap_or_else(|| t("unknown", "未知").to_string());
    format!(
        "{} {}, {} {}, {} {}, {} {}, {} {}, {} {}",
        t("calls", "调用"),
        format_number(summary.call_count as u64),
        t("results", "结果"),
        format_number(summary.result_count as u64),
        t("pending", "待完成"),
        format_number(summary.pending_count as u64),
        t("errors", "错误"),
        format_number(summary.error_count as u64),
        t("replacements", "替换"),
        format_number(summary.replacement_count as u64),
        t("latest", "最近"),
        format!("{latest_tool}/{latest_status}")
    )
}

/// 格式化工具调用状态。
///
/// 参数:
/// - `status`: 工具调用状态
///
/// 返回:
/// - 可显示状态文本
fn format_tool_status(status: &crate::state::ToolCallStatus) -> String {
    match status {
        crate::state::ToolCallStatus::Pending => t("pending", "待完成"),
        crate::state::ToolCallStatus::Completed => t("completed", "已完成"),
        crate::state::ToolCallStatus::Error => t("error", "错误"),
        crate::state::ToolCallStatus::Interrupted => t("interrupted", "已中断"),
    }
    .to_string()
}

/// 格式化 Context Epoch 摘要。
///
/// 参数:
/// - `epoch`: Context Epoch 摘要
///
/// 返回:
/// - 可显示的摘要文本
fn format_context_epoch(epoch: &crate::state::ContextEpochSummary) -> String {
    let short_hash = epoch
        .baseline_hash
        .get(..8)
        .unwrap_or(epoch.baseline_hash.as_str());
    let mut summary = format!(
        "{} {}, {} {}, {} {}",
        t("hash", "哈希"),
        short_hash,
        t("sources", "来源"),
        format_number(epoch.source_count as u64),
        t("reason", "原因"),
        epoch.last_change_reason
    );
    if let Some(blocked_source) = &epoch.blocked_source {
        summary.push_str(&format!(
            ", {} {}",
            t("blocked", "阻塞来源"),
            blocked_source
        ));
    }
    summary
}

/// 格式化动态上下文来源。
///
/// 参数:
/// - `sources`: 动态上下文来源列表
///
/// 返回:
/// - 可显示的动态上下文来源摘要
fn format_dynamic_sources(
    sources: &[crate::state::request_projection::DynamicContextSource],
) -> String {
    sources
        .iter()
        .map(|source| {
            format!(
                "{} {} {}",
                source.key,
                format_number(source.chars as u64),
                t("chars", "字符")
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// 格式化累计用量。
///
/// 参数:
/// - `requests`: 请求次数
/// - `prompt_tokens`: 输入 token 数
/// - `completion_tokens`: 输出 token 数
/// - `total_tokens`: 总 token 数
///
/// 返回:
/// - 可显示的累计用量文本
fn format_total_usage(
    requests: u64,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
) -> String {
    format!(
        "{} {}, {} {}, {} {}, {} {}",
        format_number(requests),
        t("requests", "次请求"),
        format_number(prompt_tokens),
        t("prompt", "输入"),
        format_number(completion_tokens),
        t("completion", "输出"),
        format_number(total_tokens),
        t("total", "总计")
    )
}

/// 格式化最近一次 provider 用量。
///
/// 参数:
/// - `prompt_tokens`: 输入 token 数
/// - `completion_tokens`: 输出 token 数
/// - `total_tokens`: 总 token 数
///
/// 返回:
/// - 可显示的最近用量文本
fn format_last_usage(prompt_tokens: u64, completion_tokens: u64, total_tokens: u64) -> String {
    format!(
        "{} {} / {} {} / {} {}",
        format_number(prompt_tokens),
        t("prompt", "输入"),
        format_number(completion_tokens),
        t("completion", "输出"),
        format_number(total_tokens),
        t("total", "总计")
    )
}

/// 格式化单行内容。
///
/// 参数:
/// - `label`: 行标签
/// - `value`: 行内容
///
/// 返回:
/// - 单行内容
fn row_content(label: &str, value: &str) -> String {
    format!("{} {}", pad_right(label, LABEL_WIDTH), value)
}

/// 格式化摘要行。
///
/// 参数:
/// - `label`: 行标签
/// - `value`: 行内容
/// - `width`: 内容宽度
///
/// 返回:
/// - 带边框的摘要行
fn format_row(label: &str, value: &str, width: usize) -> String {
    let content = row_content(label, value);
    format!(
        "│ {}{} │\n",
        content,
        spaces(width - visible_width(&content))
    )
}

/// 渲染顶部边框。
///
/// 参数:
/// - `title`: 卡片标题
/// - `width`: 内容宽度
///
/// 返回:
/// - 顶部边框
fn top_border(title: &str, width: usize) -> String {
    let title_width = visible_width(title);
    let right = width.saturating_sub(title_width);
    format!("┌{title}{}┐\n", "─".repeat(right + 2))
}

/// 渲染底部边框。
///
/// 参数:
/// - `width`: 内容宽度
///
/// 返回:
/// - 底部边框
fn bottom_border(width: usize) -> String {
    format!("└{}┘", "─".repeat(width + 2))
}

/// 按显示宽度右侧补空格。
///
/// 参数:
/// - `value`: 原始文本
/// - `width`: 目标显示宽度
///
/// 返回:
/// - 补齐后的文本
fn pad_right(value: &str, width: usize) -> String {
    format!(
        "{value}{}",
        spaces(width.saturating_sub(visible_width(value)))
    )
}

/// 返回指定数量空格。
///
/// 参数:
/// - `count`: 空格数量
///
/// 返回:
/// - 空格文本
fn spaces(count: usize) -> String {
    " ".repeat(count)
}

/// 计算终端显示宽度。
///
/// 参数:
/// - `value`: 原始文本
///
/// 返回:
/// - 显示宽度
fn visible_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

/// 格式化整数。
///
/// 参数:
/// - `value`: 原始整数
///
/// 返回:
/// - 带千分位分隔的整数
fn format_number(value: u64) -> String {
    let raw = value.to_string();
    let mut output = String::new();
    for (index, ch) in raw.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            output.push(',');
        }
        output.push(ch);
    }
    output.chars().rev().collect()
}
