// ============================================================
// 代码块：语言识别、高亮、外壳（语言标签 + 复制按钮）
// ============================================================

import { showToast } from "../dom.js";

/**
 * 统计文本中代码围栏 ``` 的数量，用于判断代码块是否闭合
 * @param {string} text - 原始文本
 * @returns {number} 围栏数量
 */
export function getCodeFenceCount(text) {
  if (!text) return 0;
  return (text.match(/```/g) || []).length;
}

/**
 * 获取代码块的语言名
 * @param {HTMLElement} codeEl - code 元素
 * @returns {string|null} 语言名，无法识别时返回 null
 */
export function getCodeBlockLanguage(codeEl) {
  if (!codeEl) return null;
  const classes = Array.from(codeEl.classList || []);
  const langClass = classes.find((name) => name.startsWith("language-"));
  if (langClass) return langClass.replace(/^language-/, "").toLowerCase();
  const rawClass = classes.find(
    (name) => !["hljs", "mermaid-raw", "mermaid-source-text"].includes(name)
  );
  return rawClass ? rawClass.toLowerCase() : null;
}

/**
 * 判断代码块是否已闭合
 * @param {HTMLElement} codeEl - code 元素
 * @returns {boolean} 是否已闭合
 */
export function isCodeBlockClosed(codeEl) {
  const pre = codeEl.parentElement;
  if (!pre || pre.tagName !== "PRE") return false;
  // 1. pre 后还有兄弟节点，说明已输出后续内容，代码块必然闭合
  if (pre.nextSibling) return true;
  // 2. 从消息容器 data-raw 统计围栏数，偶数即全部闭合
  const container = codeEl.closest("[data-raw]");
  if (container) {
    const count = getCodeFenceCount(container.getAttribute("data-raw") || "");
    if (count > 0 && count % 2 === 0) return true;
  }
  return false;
}

function createCodeCopyButton() {
  const btn = document.createElement("span");
  btn.className = "code-copy-btn";
  btn.innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="8" y="8" width="12" height="12" rx="2"></rect><path d="M16 4h-8a4 4 0 0 0-4 4v8"></path></svg> 复制';
  return btn;
}

function bindCodeCopy(wrapper, codeEl) {
  const btn = wrapper.querySelector(".code-copy-btn");
  if (!btn) return;
  btn.onclick = () => {
    const text = codeEl.textContent;
    navigator.clipboard
      .writeText(text)
      .then(() => {
        btn.classList.add("copied");
        btn.innerHTML =
          '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"></polyline></svg> 已复制';
        setTimeout(() => {
          btn.classList.remove("copied");
          btn.innerHTML =
            '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="8" y="8" width="12" height="12" rx="2"></rect><path d="M16 4h-8a4 4 0 0 0-4 4v8"></path></svg> 复制';
        }, 2000);
      })
      .catch(() => showToast("复制失败"));
  };
}

/**
 * 为代码块构造外壳：语言标签 + 复制按钮
 * @param {HTMLElement} codeEl - code 元素
 * @returns {void}
 */
export function ensureCodeBlockShell(codeEl) {
  const pre = codeEl && codeEl.parentElement;
  if (!pre || pre.tagName !== "PRE") return;
  if (pre.closest(".mermaid-code") || pre.closest(".mermaid-wrapper")) return;
  // 1. 复用或新建 wrapper
  let wrapper =
    pre.parentElement && pre.parentElement.classList.contains("code-block-wrapper")
      ? pre.parentElement
      : null;
  if (!wrapper) {
    wrapper = document.createElement("div");
    wrapper.className = "code-block-wrapper";
    pre.parentNode.insertBefore(wrapper, pre);
    wrapper.appendChild(pre);
  }
  // 2. 复用或新建 header
  let header = wrapper.querySelector(".code-block-header");
  if (!header) {
    header = document.createElement("div");
    header.className = "code-block-header";
    wrapper.insertBefore(header, pre);
  }
  // 3. 语言标签
  let langLabel = header.querySelector(".code-lang");
  if (!langLabel) {
    langLabel = document.createElement("span");
    langLabel.className = "code-lang";
    header.insertBefore(langLabel, header.firstChild);
  }
  langLabel.textContent = getCodeBlockLanguage(codeEl) || "代码";
  // 4. 复制按钮
  if (!header.querySelector(".code-copy-btn")) {
    header.appendChild(createCodeCopyButton());
  }
  bindCodeCopy(wrapper, codeEl);
}

/**
 * 为容器内所有代码块渲染外壳
 * @param {HTMLElement} container - DOM 容器
 * @returns {void}
 */
export function renderCodeBlockShells(container) {
  if (!container) return;
  const codeEls = Array.from(
    container.querySelectorAll("pre > code:not(.mermaid-source-text)")
  );
  for (const el of codeEls) {
    const pre = el.parentElement;
    if (!pre || pre.closest(".mermaid-code") || pre.closest(".mermaid-wrapper")) continue;
    ensureCodeBlockShell(el);
  }
}

/**
 * 高亮容器内未着色的代码块
 * @param {HTMLElement} container - 待处理的 DOM 容器
 * @param {boolean} skipLast - 流式模式下是否跳过最后未闭合代码块
 * @returns {void}
 */
export function highlightCodeBlocks(container, skipLast = false) {
  if (!container || !window.hljs) return;
  const codeEls = Array.from(
    container.querySelectorAll("pre > code:not(.hljs):not(.mermaid-source-text)")
  );
  for (let i = 0; i < codeEls.length; i++) {
    const el = codeEls[i];
    const pre = el.parentElement;
    if (!pre || pre.closest(".mermaid-code") || pre.closest(".mermaid-wrapper")) continue;
    // 1. 流式模式下跳过最后未闭合代码块
    if (skipLast && i === codeEls.length - 1 && !isCodeBlockClosed(el)) continue;
    const lang = getCodeBlockLanguage(el);
    try {
      if (lang && window.hljs.getLanguage(lang)) {
        el.innerHTML = window.hljs.highlight(el.textContent, { language: lang }).value;
      } else {
        el.innerHTML = window.hljs.highlightAuto(el.textContent).value;
      }
      el.classList.add("hljs");
    } catch (err) {
      continue;
    }
    // 2. 闭合后启用外壳
    ensureCodeBlockShell(el);
  }
}
