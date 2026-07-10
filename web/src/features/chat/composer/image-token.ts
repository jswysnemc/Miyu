export const IMAGE_TOKEN_RE = /\[IMAGE-(\d+)\]/g;

export type ImageTokenRange = {
  start: number;
  end: number;
  token: string;
};

export type AtomicDeleteResult = {
  value: string;
  caret: number;
};

/**
 * 收集文本中所有图片 token 的位置。
 *
 * @param text 输入文本
 * @returns 图片 token 区间
 */
export function collectImageTokenRanges(text: string): ImageTokenRange[] {
  const ranges: ImageTokenRange[] = [];
  const pattern = new RegExp(IMAGE_TOKEN_RE.source, "g");
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(text)) !== null) {
    ranges.push({ start: match.index, end: match.index + match[0].length, token: match[0] });
  }
  return ranges;
}

/**
 * 获取当前输入中最小可用的图片序号。
 *
 * @param text 输入文本
 * @param existingTokens 已存在的附件 token
 * @returns 新图片 token
 */
export function nextImageToken(text: string, existingTokens: string[]): string {
  const used = new Set<number>();
  for (const range of collectImageTokenRanges(text)) used.add(Number(range.token.slice(7, -1)));
  for (const token of existingTokens) {
    const match = /^\[IMAGE-(\d+)\]$/.exec(token);
    if (match) used.add(Number(match[1]));
  }
  let index = 1;
  while (used.has(index)) index += 1;
  return `[IMAGE-${index}]`;
}

/**
 * 在选区处插入图片 token，并补齐必要空格。
 *
 * @param text 输入文本
 * @param selectionStart 选区起点
 * @param selectionEnd 选区终点
 * @param token 图片 token
 * @returns 新文本和光标位置
 */
export function insertImageToken(
  text: string,
  selectionStart: number,
  selectionEnd: number,
  token: string
): AtomicDeleteResult {
  const before = text.slice(0, selectionStart);
  const after = text.slice(selectionEnd);
  const leading = before && !/\s$/.test(before) ? " " : "";
  const trailing = after && !/^\s/.test(after) ? " " : "";
  const insertion = `${leading}${token}${trailing}`;
  return { value: before + insertion + after, caret: before.length + insertion.length };
}

/**
 * 从输入文本中完整移除指定图片 token。
 *
 * @param text 输入文本
 * @param token 图片 token
 * @returns 清理多余空格后的文本
 */
export function removeImageToken(text: string, token: string): string {
  const escaped = token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return text
    .replace(new RegExp(`\\s*${escaped}\\s*`, "g"), " ")
    .replace(/[ \t]{2,}/g, " ")
    .replace(/ ?\n ?/g, "\n")
    .trimStart();
}

/**
 * 对 Backspace、Delete 或与 token 相交的选区执行原子删除。
 *
 * @param text 输入文本
 * @param selectionStart 选区起点
 * @param selectionEnd 选区终点
 * @param key 删除按键
 * @returns 已处理时返回新文本和光标，否则返回空值
 */
export function deleteImageTokenAtomically(
  text: string,
  selectionStart: number,
  selectionEnd: number,
  key: "Backspace" | "Delete"
): AtomicDeleteResult | null {
  const ranges = collectImageTokenRanges(text);
  if (selectionStart !== selectionEnd) {
    let start = selectionStart;
    let end = selectionEnd;
    let intersected = false;
    for (const range of ranges) {
      if (start < range.end && end > range.start) {
        intersected = true;
        start = Math.min(start, range.start);
        end = Math.max(end, range.end);
      }
    }
    return intersected ? { value: text.slice(0, start) + text.slice(end), caret: start } : null;
  }

  const hit = ranges.find((range) => key === "Backspace"
    ? selectionStart > range.start && selectionStart <= range.end
    : selectionStart >= range.start && selectionStart < range.end);
  if (!hit) return null;
  let start = hit.start;
  let end = hit.end;
  if (start > 0 && text[start - 1] === " " && (end >= text.length || /[ \n]/.test(text[end]))) start -= 1;
  else if (end < text.length && text[end] === " " && (start === 0 || /[ \n]/.test(text[start - 1]))) end += 1;
  return { value: text.slice(0, start) + text.slice(end), caret: start };
}
