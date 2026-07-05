// ============================================================
// 数学公式：占位保护 + KaTeX 渲染
// marked.parse 前用占位符替换公式，避免定界符被破坏
// ============================================================

const MATH_DELIMITER_CONFIGS = [
  { left: "$$", right: "$$", displayMode: true },
  { left: "\\[", right: "\\]", displayMode: true },
  { left: "\\(", right: "\\)", displayMode: false },
  { left: "$", right: "$", displayMode: false },
];

const MATH_IGNORED_SELECTOR =
  "script,noscript,style,textarea,pre,code,option,svg,.katex,.math-render,.mermaid-wrapper,.mermaid-code";

/**
 * 剥离包裹公式的反引号，使 $...$ / $$...$$ 暴露给公式解析器
 * @param {string} text - 原始 Markdown 文本
 * @returns {string} 剥离反引号后的文本
 */
export function unwrapMathFromCode(text) {
  if (!text) return text;
  return text
    .replace(/`(\$\$[^`]*?\$\$)`/g, "$1")
    .replace(/`(\$[^`]*?\$)`/g, "$1");
}

/**
 * 提取文本中的数学公式并替换为占位符，防止 marked.parse 破坏定界符
 * @param {string} text - 原始 Markdown 文本
 * @returns {{text: string, mathMap: string[]}} 占位后的文本与公式映射表
 */
export function protectMath(text) {
  if (!text) return { text, mathMap: [] };
  const mathMap = [];
  const pattern = /(\$\$[\s\S]*?\$\$|\\\[[\s\S]*?\\\]|\\\([\s\S]*?\\\)|\$[^\n$]*?\$)/g;
  const protectedText = text.replace(pattern, (match) => {
    const idx = mathMap.length;
    mathMap.push(match);
    return "@@MATH" + idx + "@@";
  });
  return { text: protectedText, mathMap };
}

/**
 * 在 DOM 容器的文本节点中还原被占位的数学公式
 * @param {HTMLElement} container - 已设置 innerHTML 的 DOM 容器
 * @param {string[]} mathMap - protectMath 返回的公式映射表
 * @returns {void}
 */
export function restoreMathPlaceholders(container, mathMap) {
  if (!container || !mathMap || mathMap.length === 0) return;
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      return /@@MATH\d+@@/.test(node.nodeValue || "")
        ? NodeFilter.FILTER_ACCEPT
        : NodeFilter.FILTER_REJECT;
    },
  });
  const nodes = [];
  let node = walker.nextNode();
  while (node) {
    nodes.push(node);
    node = walker.nextNode();
  }
  nodes.forEach((n) => {
    n.nodeValue = n.nodeValue.replace(/@@MATH(\d+)@@/g, (m, idx) => mathMap[parseInt(idx)] || m);
  });
}

function isEscapedDelimiter(text, index) {
  let slashCount = 0;
  for (let i = index - 1; i >= 0 && text[i] === "\\"; i--) slashCount++;
  return slashCount % 2 === 1;
}

function findUnescapedDelimiter(text, delimiter, startIndex) {
  let idx = text.indexOf(delimiter, startIndex);
  while (idx !== -1) {
    if (!isEscapedDelimiter(text, idx)) return idx;
    idx = text.indexOf(delimiter, idx + delimiter.length);
  }
  return -1;
}

function findNextMathCandidate(text, startIndex) {
  let best = null;
  for (const config of MATH_DELIMITER_CONFIGS) {
    let searchFrom = startIndex;
    while (searchFrom < text.length) {
      const leftIndex = findUnescapedDelimiter(text, config.left, searchFrom);
      if (leftIndex === -1) break;
      // 1. 单 $ 遇到 $$ 时跳过，交给 $$ 配置处理
      if (config.left === "$" && text[leftIndex + 1] === "$") {
        searchFrom = leftIndex + 1;
        continue;
      }
      const contentStart = leftIndex + config.left.length;
      const rightIndex = findUnescapedDelimiter(text, config.right, contentStart);
      if (rightIndex === -1) break;
      const tex = text.slice(contentStart, rightIndex);
      if (!tex.trim()) {
        searchFrom = contentStart;
        continue;
      }
      const candidate = {
        start: leftIndex,
        end: rightIndex + config.right.length,
        tex,
        displayMode: config.displayMode,
      };
      if (
        !best ||
        candidate.start < best.start ||
        (candidate.start === best.start && candidate.end > best.end)
      ) {
        best = candidate;
      }
      break;
    }
  }
  return best;
}

function createMathRenderElement(tex, displayMode) {
  const el = document.createElement("span");
  el.className = displayMode
    ? "math-render math-render-block"
    : "math-render math-render-inline";
  try {
    window.katex.render(tex, el, { displayMode, throwOnError: false });
    el.setAttribute("data-katex", "1");
    return el;
  } catch (err) {
    return null;
  }
}

function hasFollowingRenderedNode(root, node) {
  let cur = node;
  while (cur && cur !== root) {
    if (cur.nextSibling) return true;
    cur = cur.parentNode;
  }
  return false;
}

/**
 * 渲染容器内文本节点中的数学公式
 * @param {HTMLElement} container - 待渲染的 DOM 容器
 * @param {boolean} skipLast - 流式模式下是否跳过末尾仍可能继续写入的内容
 * @returns {void}
 */
export function renderMath(container, skipLast = false) {
  if (!container || !window.katex) return;
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const parent = node.parentElement;
      if (!parent || parent.closest(MATH_IGNORED_SELECTOR)) return NodeFilter.FILTER_REJECT;
      if (!/[\\$]/.test(node.nodeValue || "")) return NodeFilter.FILTER_REJECT;
      if (skipLast && !hasFollowingRenderedNode(container, node)) return NodeFilter.FILTER_REJECT;
      return findNextMathCandidate(node.nodeValue || "", 0)
        ? NodeFilter.FILTER_ACCEPT
        : NodeFilter.FILTER_REJECT;
    },
  });
  const nodes = [];
  let node = walker.nextNode();
  while (node) {
    nodes.push(node);
    node = walker.nextNode();
  }
  nodes.forEach((textNode) => {
    const text = textNode.nodeValue || "";
    const fragment = document.createDocumentFragment();
    let cursor = 0;
    let changed = false;
    let candidate = findNextMathCandidate(text, cursor);
    while (candidate) {
      if (candidate.start > cursor) {
        fragment.appendChild(document.createTextNode(text.slice(cursor, candidate.start)));
      }
      const mathEl = createMathRenderElement(candidate.tex, candidate.displayMode);
      if (mathEl) {
        fragment.appendChild(mathEl);
        changed = true;
      } else {
        fragment.appendChild(document.createTextNode(text.slice(candidate.start, candidate.end)));
      }
      cursor = candidate.end;
      candidate = findNextMathCandidate(text, cursor);
    }
    if (!changed) return;
    if (cursor < text.length) {
      fragment.appendChild(document.createTextNode(text.slice(cursor)));
    }
    textNode.parentNode.replaceChild(fragment, textNode);
  });
}
