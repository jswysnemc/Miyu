import { useEffect, useRef } from "react";
import type { ChangeEvent, ClipboardEvent, KeyboardEvent, UIEvent } from "react";
import { collectImageTokenRanges, deleteImageTokenAtomically } from "./image-token";
import { isCursorOnFirstLine, isCursorOnLastLine, navigateInputHistory } from "./input-history";
import type { InputHistoryState } from "./input-history";

type ComposerTextareaProps = {
  value: string;
  historyEntries: string[];
  disabled: boolean;
  placeholder: string;
  onChange: (value: string) => void;
  onAttachmentsSync: (value: string) => void;
  onPasteImages: (files: File[], selectionStart: number, selectionEnd: number) => Promise<number | undefined>;
  onSubmit: () => void;
};

/**
 * 渲染支持图片 token、高亮、历史切换和原子删除的输入框。
 *
 * @param props 输入内容、历史记录、附件回调和提交回调
 * @returns 聊天文本输入框
 */
export function ComposerTextarea(props: ComposerTextareaProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const highlightRef = useRef<HTMLDivElement>(null);
  const historyRef = useRef<InputHistoryState>({ index: null, draft: "" });
  const lastEscapeRef = useRef(0);

  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = "24px";
    textarea.style.height = `${Math.min(textarea.scrollHeight, 150)}px`;
    textarea.style.overflowY = textarea.scrollHeight > 150 ? "auto" : "hidden";
    if (highlightRef.current) highlightRef.current.style.height = textarea.style.height;
  }, [props.value]);

  /**
   * 更新文本并退出历史浏览状态。
   *
   * @param event 文本输入事件
   */
  const handleChange = (event: ChangeEvent<HTMLTextAreaElement>) => {
    historyRef.current = { index: null, draft: "" };
    props.onChange(event.target.value);
    props.onAttachmentsSync(event.target.value);
  };

  /**
   * 同步文本框与 token 高亮层的滚动位置。
   *
   * @param event 文本框滚动事件
   */
  const handleScroll = (event: UIEvent<HTMLTextAreaElement>) => {
    if (!highlightRef.current) return;
    highlightRef.current.scrollTop = event.currentTarget.scrollTop;
    highlightRef.current.scrollLeft = event.currentTarget.scrollLeft;
  };

  /**
   * 粘贴剪贴板中的全部图片，并在当前选区插入 token。
   *
   * @param event 剪贴板事件
   */
  const handlePaste = (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const files = Array.from(event.clipboardData.files).filter((file) => file.type.startsWith("image/"));
    if (files.length === 0) return;
    event.preventDefault();
    const start = event.currentTarget.selectionStart;
    const end = event.currentTarget.selectionEnd;
    void props.onPasteImages(files, start, end).then((caret) => {
      if (caret === undefined) return;
      requestAnimationFrame(() => setTextareaSelection(textareaRef.current, caret));
    });
  };

  /**
   * 处理 token 原子删除、输入历史、双击 Escape 清空和回车提交。
   *
   * @param event 文本框键盘事件
   */
  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    const textarea = event.currentTarget;
    if (event.key === "Backspace" || event.key === "Delete") {
      const deleted = deleteImageTokenAtomically(
        props.value,
        textarea.selectionStart,
        textarea.selectionEnd,
        event.key
      );
      if (deleted) {
        event.preventDefault();
        props.onChange(deleted.value);
        props.onAttachmentsSync(deleted.value);
        requestAnimationFrame(() => setTextareaSelection(textareaRef.current, deleted.caret));
        return;
      }
    }

    if (event.key === "ArrowUp" && isCursorOnFirstLine(props.value, textarea.selectionStart)) {
      if (applyHistoryNavigation("up", props, historyRef, textareaRef)) event.preventDefault();
      return;
    }
    if (event.key === "ArrowDown" && isCursorOnLastLine(props.value, textarea.selectionEnd)) {
      if (applyHistoryNavigation("down", props, historyRef, textareaRef)) event.preventDefault();
      return;
    }
    if (event.key === "Escape") {
      const now = Date.now();
      if (now - lastEscapeRef.current < 500 && props.value) {
        event.preventDefault();
        props.onChange("");
        props.onAttachmentsSync("");
        historyRef.current = { index: null, draft: "" };
      }
      lastEscapeRef.current = now;
      return;
    }
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      props.onSubmit();
    }
  };

  return (
    <div className="composer-text-wrap">
      <div className="composer-highlight" ref={highlightRef} aria-hidden="true">
        <HighlightedInput value={props.value} />
      </div>
      <textarea
        ref={textareaRef}
        value={props.value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onPaste={handlePaste}
        onScroll={handleScroll}
        placeholder={props.placeholder}
        disabled={props.disabled}
        rows={1}
      />
    </div>
  );
}

/**
 * 将输入文本拆成普通文本和图片 token 片段。
 *
 * @param props 当前输入值
 * @returns 与文本框排版一致的高亮内容
 */
function HighlightedInput({ value }: { value: string }) {
  const ranges = collectImageTokenRanges(value);
  const fragments = [];
  let cursor = 0;
  for (const range of ranges) {
    fragments.push(<span key={`text-${cursor}`} className="input-plain-text">{value.slice(cursor, range.start)}</span>);
    fragments.push(<mark key={`${range.start}-${range.token}`} className="input-image-token">{range.token}</mark>);
    cursor = range.end;
  }
  fragments.push(<span key={`text-${cursor}`} className="input-plain-text">{value.slice(cursor)}</span>);
  return <>{fragments}{value.endsWith("\n") ? "\n " : ""}</>;
}

/**
 * 应用一次输入历史导航并安排光标位置。
 *
 * @param direction 历史移动方向
 * @param props 输入框属性
 * @param historyRef 历史游标引用
 * @param textareaRef 文本框引用
 * @returns 是否切换了历史输入
 */
function applyHistoryNavigation(
  direction: "up" | "down",
  props: ComposerTextareaProps,
  historyRef: React.MutableRefObject<InputHistoryState>,
  textareaRef: React.RefObject<HTMLTextAreaElement | null>
): boolean {
  const result = navigateInputHistory(props.historyEntries, historyRef.current, props.value, direction);
  if (!result) return false;
  historyRef.current = result.state;
  props.onChange(result.value);
  props.onAttachmentsSync(result.value);
  requestAnimationFrame(() => {
    const caret = direction === "up" ? 0 : result.value.length;
    setTextareaSelection(textareaRef.current, caret);
  });
  return true;
}

/**
 * 聚焦文本框并设置折叠选区。
 *
 * @param textarea 文本框元素
 * @param caret 光标位置
 */
function setTextareaSelection(textarea: HTMLTextAreaElement | null, caret: number) {
  if (!textarea) return;
  textarea.focus();
  textarea.setSelectionRange(caret, caret);
}
