import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import type { ChangeEvent, ClipboardEvent, KeyboardEvent } from "react";
import { isCursorOnFirstLine, isCursorOnLastLine, navigateInputHistory } from "./input-history";
import type { InputHistoryState } from "./input-history";
import { FileMentionPopover } from "./file-mention-popover";

type ComposerTextareaProps = {
  value: string;
  historyEntries: string[];
  disabled: boolean;
  placeholder: string;
  onChange: (value: string) => void;
  onPasteImages: (files: File[], selectionStart: number, selectionEnd: number) => Promise<number | undefined>;
  onSubmit: () => void;
};

export type ComposerTextareaHandle = {
  openMentionPicker: () => void;
};

/**
 * 渲染支持图片粘贴、历史切换和 @ 文件引用的输入框。
 *
 * @param props 输入内容、历史记录、附件回调和提交回调
 * @param ref 暴露 openMentionPicker 的句柄
 * @returns 聊天文本输入框
 */
export const ComposerTextarea = forwardRef<ComposerTextareaHandle, ComposerTextareaProps>(function ComposerTextarea(props, ref) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const historyRef = useRef<InputHistoryState>({ index: null, draft: "" });
  const lastEscapeRef = useRef(0);
  const [mentionOpen, setMentionOpen] = useState(false);
  const mentionRangeRef = useRef<{ start: number; end: number } | null>(null);

  useImperativeHandle(ref, () => ({
    /** 以当前选区为插入点打开文件引用浮层。 */
    openMentionPicker: () => {
      const textarea = textareaRef.current;
      const caret = textarea ? textarea.selectionStart : props.value.length;
      mentionRangeRef.current = { start: caret, end: textarea ? textarea.selectionEnd : caret };
      setMentionOpen(true);
    }
  }));

  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = "24px";
    textarea.style.height = `${Math.min(textarea.scrollHeight, 150)}px`;
    textarea.style.overflowY = textarea.scrollHeight > 150 ? "auto" : "hidden";
  }, [props.value]);

  /**
   * 更新文本、退出历史浏览，并在输入 @ 时打开文件引用浮层。
   *
   * @param event 文本输入事件
   */
  const handleChange = (event: ChangeEvent<HTMLTextAreaElement>) => {
    historyRef.current = { index: null, draft: "" };
    const next = event.target.value;
    const caret = event.target.selectionStart;
    // 1. 仅在本次输入新增了一个 @ 字符时触发浮层
    if (next.length === props.value.length + 1 && next[caret - 1] === "@") {
      mentionRangeRef.current = { start: caret - 1, end: caret };
      setMentionOpen(true);
    }
    props.onChange(next);
  };

  /**
   * 在触发位置插入文件引用并删除触发的 @ 字符。
   *
   * @param path 选中的文件相对路径
   */
  const handleMentionSelect = (path: string) => {
    const range = mentionRangeRef.current ?? { start: props.value.length, end: props.value.length };
    const insertion = `@${path} `;
    props.onChange(`${props.value.slice(0, range.start)}${insertion}${props.value.slice(range.end)}`);
    closeMention();
    requestAnimationFrame(() => setTextareaSelection(textareaRef.current, range.start + insertion.length));
  };

  /** 关闭文件引用浮层并把焦点交还文本框。 */
  const closeMention = () => {
    mentionRangeRef.current = null;
    setMentionOpen(false);
    requestAnimationFrame(() => textareaRef.current?.focus());
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
   * 处理输入历史、双击 Escape 清空和回车提交。
   *
   * @param event 文本框键盘事件
   */
  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    const textarea = event.currentTarget;
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
      <FileMentionPopover open={mentionOpen} onSelect={handleMentionSelect} onClose={closeMention} />
      <textarea
        ref={textareaRef}
        value={props.value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onPaste={handlePaste}
        placeholder={props.placeholder}
        disabled={props.disabled}
        rows={1}
      />
    </div>
  );
});

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
