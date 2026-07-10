import { Check, Copy } from "lucide-react";
import { useEffect, useRef, useState } from "react";

type MessageActionsProps = {
  text: string;
  timestamp?: string;
};

/**
 * 消息操作行，包含可选时间戳和复制原文按钮。
 *
 * @param props text 为待复制原文，timestamp 为可选消息时间
 * @returns 悬停显示的操作行
 */
export function MessageActions({ text, timestamp }: MessageActionsProps) {
  const [copied, setCopied] = useState(false);
  const timerRef = useRef<number | null>(null);

  // 1. 组件卸载时清理复制状态定时器
  useEffect(() => () => {
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
  }, []);

  // 2. 复制原文并短暂切换为成功图标
  const onCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
      timerRef.current = window.setTimeout(() => setCopied(false), 1_600);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div className="message-actions">
      {timestamp && <time className="message-timestamp">{formatTimestamp(timestamp)}</time>}
      <button type="button" className="message-copy" onClick={onCopy} aria-label="复制消息原文" title="复制原文">
        {copied ? <Check size={13} /> : <Copy size={13} />}
      </button>
    </div>
  );
}

/**
 * 把 ISO 时间格式化为本地时分显示。
 *
 * @param value ISO 时间文本
 * @returns 本地化时间，无法解析时返回原文
 */
function formatTimestamp(value: string): string {
  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed)) return value;
  const date = new Date(parsed);
  const today = new Date();
  const sameDay = date.toDateString() === today.toDateString();
  const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return sameDay ? time : `${date.toLocaleDateString()} ${time}`;
}
