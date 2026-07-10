import { useQuery } from "@tanstack/react-query";
import { Activity, Cpu, Gauge, HardDrive, TerminalSquare } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { api } from "../../api/client";
import "./system-usage.css";

/**
 * 渲染顶栏系统用量入口和详情浮层。
 *
 * @returns 系统用量组件
 */
export function SystemUsage() {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const usage = useQuery({
    queryKey: ["system-usage"],
    queryFn: api.system.usage,
    refetchInterval: 5_000
  });
  const contextPercent = Math.round(Math.min(1, Math.max(0, usage.data?.session.context_token_ratio ?? 0)) * 100);

  useEffect(() => {
    /** 在用量浮层外点击时关闭浮层。 */
    const handlePointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, []);

  return (
    <div className="system-usage" ref={rootRef}>
      <button type="button" className="system-usage-trigger" onClick={() => setOpen((value) => !value)} aria-expanded={open} aria-label="查看系统用量">
        <span className="usage-ring" style={{ background: `conic-gradient(var(--signal) ${contextPercent}%, var(--paper-deep) 0)` }}><Gauge size={12} /></span>
        <span><strong>{usage.data ? formatTokenCount(usage.data.session.context_prompt_tokens) : "--"}</strong><small>{contextPercent}% 上下文</small></span>
      </button>
      {open && (
        <div className="system-usage-popover">
          <header><div><span>系统用量</span><strong>当前会话与进程</strong></div><i className={usage.data?.runtime.active_run ? "active" : ""} /></header>
          {usage.isLoading && <div className="usage-loading">正在读取用量</div>}
          {usage.error && <div className="usage-error">{usage.error.message}</div>}
          {usage.data && (
            <>
              <section className="context-usage-card">
                <div className="context-usage-head"><span>上下文占用</span><strong>{contextPercent}%</strong></div>
                <div className="context-usage-track"><span style={{ width: `${contextPercent}%` }} /></div>
                <small>{formatTokenCount(usage.data.session.context_prompt_tokens)} / {formatTokenCount(usage.data.session.context_window_tokens)} token</small>
              </section>
              <div className="usage-metric-grid">
                <UsageMetric icon={<Activity size={14} />} label="累计 Token" value={formatTokenCount(usage.data.session.total_tokens)} detail={`${usage.data.session.requests} 次请求`} />
                <UsageMetric icon={<Cpu size={14} />} label="进程 CPU" value={`${usage.data.process.cpu_percent.toFixed(1)}%`} detail={`PID ${usage.data.process.pid}`} />
                <UsageMetric icon={<HardDrive size={14} />} label="常驻内存" value={formatBytes(usage.data.process.rss_bytes)} detail={formatDuration(usage.data.process.uptime_seconds)} />
                <UsageMetric icon={<TerminalSquare size={14} />} label="运行时" value={`${usage.data.runtime.terminal_count} 个终端`} detail={usage.data.runtime.active_run ? "Agent 正在运行" : "Agent 空闲"} />
              </div>
              <div className="usage-token-breakdown"><span>输入 {formatTokenCount(usage.data.session.prompt_tokens)}</span><span>输出 {formatTokenCount(usage.data.session.completion_tokens)}</span><span>工具 {usage.data.session.tool_calls}</span><span>轮次 {usage.data.session.turn_count}</span></div>
            </>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * 渲染单个系统用量指标。
 *
 * @param props 图标、名称、数值和说明
 * @returns 指标卡片
 */
function UsageMetric({ icon, label, value, detail }: { icon: React.ReactNode; label: string; value: string; detail: string }) {
  return <div className="usage-metric"><span>{icon}</span><div><small>{label}</small><strong>{value}</strong><i>{detail}</i></div></div>;
}

/**
 * 格式化较大的计数。
 *
 * @param value 原始数值
 * @returns 紧凑计数文本
 */
export function formatTokenCount(value: number): string {
  if (value >= 1_000_000) return `${stripTrailingZero(value / 1_000_000)}m`;
  if (value >= 1_000) return `${stripTrailingZero(value / 1_000)}k`;
  return String(value);
}

/**
 * 移除一位小数格式中的无效零。
 *
 * @param value 需要压缩显示的数值
 * @returns 最多保留一位小数的文本
 */
function stripTrailingZero(value: number): string {
  return value.toFixed(1).replace(/\.0$/, "");
}

/**
 * 格式化字节数。
 *
 * @param value 字节数
 * @returns 内存大小文本
 */
function formatBytes(value?: number | null): string {
  if (!value) return "不可用";
  const units = ["B", "KiB", "MiB", "GiB"];
  let amount = value;
  let index = 0;
  while (amount >= 1024 && index < units.length - 1) {
    amount /= 1024;
    index += 1;
  }
  return `${amount.toFixed(index > 1 ? 1 : 0)} ${units[index]}`;
}

/**
 * 格式化服务运行时间。
 *
 * @param seconds 运行秒数
 * @returns 运行时间文本
 */
function formatDuration(seconds: number): string {
  if (seconds < 60) return `运行 ${seconds} 秒`;
  if (seconds < 3_600) return `运行 ${Math.floor(seconds / 60)} 分钟`;
  return `运行 ${Math.floor(seconds / 3_600)} 小时 ${Math.floor(seconds % 3_600 / 60)} 分钟`;
}
