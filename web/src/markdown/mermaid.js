// ============================================================
// Mermaid 图表渲染：将闭合的 mermaid 代码块替换为图表预览
// ============================================================

import { showToast } from "../dom.js";
import { mermaidConfig } from "../theme.js";
import { isCodeBlockClosed } from "./code.js";

let mermaidRenderSeq = 0;

/**
 * 切换 Mermaid 包装容器的图表/源码视图
 * @param {HTMLElement} wrapper - mermaid-wrapper 元素
 * @param {"preview"|"code"} mode - 视图模式
 * @returns {void}
 */
function switchMermaidTab(wrapper, mode) {
  const btnPreview = wrapper.querySelector(".mermaid-toolbar .mermaid-tool-btn:nth-child(1)");
  const btnCode = wrapper.querySelector(".mermaid-toolbar .mermaid-tool-btn:nth-child(2)");
  const previewDiv = wrapper.querySelector(".mermaid-preview");
  const codeDiv = wrapper.querySelector(".mermaid-code");
  if (mode === "preview") {
    btnPreview && btnPreview.classList.add("active");
    btnCode && btnCode.classList.remove("active");
    if (previewDiv) previewDiv.style.display = "flex";
    if (codeDiv) codeDiv.style.display = "none";
  } else {
    btnPreview && btnPreview.classList.remove("active");
    btnCode && btnCode.classList.add("active");
    if (previewDiv) previewDiv.style.display = "none";
    if (codeDiv) codeDiv.style.display = "block";
  }
}

/**
 * 创建 Mermaid 渲染成功后的视图容器
 * @param {string} source - Mermaid 源码
 * @param {string} svg - Mermaid 渲染后的 SVG 字符串
 * @returns {HTMLElement} mermaid-wrapper 元素
 */
function createMermaidWrapper(source, svg) {
  const wrapper = document.createElement("div");
  wrapper.className = "mermaid-wrapper";

  const toolbar = document.createElement("div");
  toolbar.className = "mermaid-toolbar";

  const btnPreview = document.createElement("span");
  btnPreview.className = "mermaid-tool-btn active";
  btnPreview.textContent = "图表";
  btnPreview.onclick = () => switchMermaidTab(wrapper, "preview");

  const btnCode = document.createElement("span");
  btnCode.className = "mermaid-tool-btn";
  btnCode.textContent = "源码";
  btnCode.onclick = () => switchMermaidTab(wrapper, "code");

  const btnZoom = document.createElement("span");
  btnZoom.className = "mermaid-tool-btn";
  btnZoom.innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"/></svg>放大';
  // 1. 放大按钮派发事件，由 mermaid-viewer 监听处理
  btnZoom.onclick = () => {
    const svgEl = wrapper.querySelector(".mermaid-preview svg");
    if (svgEl) {
      window.dispatchEvent(
        new CustomEvent("miyu:mermaid-zoom", { detail: { svg: svgEl.outerHTML } })
      );
    } else {
      showToast("图表尚未渲染完成");
    }
  };

  const btnCopy = document.createElement("span");
  btnCopy.className = "mermaid-tool-copy";
  btnCopy.innerHTML =
    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="8" y="8" width="12" height="12" rx="2"></rect><path d="M16 4h-8a4 4 0 0 0-4 4v8"></path></svg> 复制';
  btnCopy.onclick = () => {
    navigator.clipboard.writeText(source).then(() => {
      btnCopy.innerHTML =
        '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"></polyline></svg> 已复制';
      setTimeout(() => {
        btnCopy.innerHTML =
          '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="8" y="8" width="12" height="12" rx="2"></rect><path d="M16 4h-8a4 4 0 0 0-4 4v8"></path></svg> 复制';
      }, 2000);
    });
  };

  const previewDiv = document.createElement("div");
  previewDiv.className = "mermaid-preview";
  const mermaidDiv = document.createElement("div");
  mermaidDiv.className = "mermaid";
  mermaidDiv.innerHTML = svg;
  previewDiv.appendChild(mermaidDiv);

  const codeDiv = document.createElement("div");
  codeDiv.className = "mermaid-code";
  codeDiv.style.display = "none";
  const preNode = document.createElement("pre");
  const codeNode = document.createElement("code");
  codeNode.className = "mermaid-source-text hljs";
  if (window.hljs) {
    try {
      codeNode.innerHTML = window.hljs.highlightAuto(source).value;
    } catch (err) {
      codeNode.textContent = source;
    }
  } else {
    codeNode.textContent = source;
  }
  preNode.appendChild(codeNode);
  codeDiv.appendChild(preNode);

  toolbar.append(btnPreview, btnCode, btnZoom, btnCopy);
  wrapper.append(toolbar, previewDiv, codeDiv);
  return wrapper;
}

/**
 * 渲染单个 Mermaid 代码块候选
 * @param {HTMLElement} codeEl - mermaid 源码 code 元素
 * @returns {void}
 */
function renderMermaidCandidate(codeEl) {
  if (!codeEl || codeEl.__mermaidRendering) return;
  const pre = codeEl.parentElement;
  if (!pre || pre.tagName !== "PRE") return;
  const source = codeEl.textContent || "";
  if (!source.trim() || codeEl.__mermaidErrorSource === source) return;
  codeEl.__mermaidRendering = true;
  // 1. 按当前主题初始化 Mermaid
  window.mermaid.initialize(mermaidConfig());
  const renderId = `mermaid-render-${Date.now()}-${mermaidRenderSeq++}`;
  let renderTask;
  try {
    renderTask = Promise.resolve(window.mermaid.render(renderId, source));
  } catch (err) {
    codeEl.__mermaidErrorSource = source;
    codeEl.__mermaidRendering = false;
    console.info("【前端/Mermaid】渲染暂缓", err);
    return;
  }
  // 2. 渲染成功后替换代码块为 wrapper
  renderTask
    .then((result) => {
      if (!pre.isConnected || codeEl.textContent !== source) return;
      const svg = typeof result === "string" ? result : result.svg;
      if (!svg) return;
      const wrapper = createMermaidWrapper(source, svg);
      if (result && typeof result.bindFunctions === "function") {
        const previewDiv = wrapper.querySelector(".mermaid-preview");
        if (previewDiv) result.bindFunctions(previewDiv);
      }
      let target = pre;
      if (pre.parentNode && pre.parentNode.classList.contains("code-block-wrapper")) {
        target = pre.parentNode;
      }
      if (!target.parentNode) return;
      target.parentNode.replaceChild(wrapper, target);
    })
    .catch((err) => {
      codeEl.__mermaidErrorSource = source;
      console.info("【前端/Mermaid】渲染暂缓", err);
    })
    .finally(() => {
      codeEl.__mermaidRendering = false;
    });
}

/**
 * 将已闭合的 Mermaid 代码块替换为图表预览
 * @param {HTMLElement} container - 待处理的消息容器
 * @param {boolean} skipLast - 是否跳过最后一个未闭合 Mermaid 代码块
 * @returns {void}
 */
export function renderMermaid(container, skipLast = false) {
  if (!container || !window.mermaid) return;
  const codeEls = Array.from(
    container.querySelectorAll("pre > code.mermaid-raw, pre > code.language-mermaid, pre > code.mermaid")
  );
  if (codeEls.length === 0) return;
  for (let i = 0; i < codeEls.length; i++) {
    const el = codeEls[i];
    const pre = el.parentElement;
    if (!pre || pre.tagName !== "PRE") continue;
    if (pre.closest(".mermaid-code") || pre.closest(".mermaid-wrapper")) continue;
    // 1. 流式模式下跳过最后未闭合的 Mermaid 块
    if (skipLast && i === codeEls.length - 1 && !isCodeBlockClosed(el)) continue;
    renderMermaidCandidate(el);
  }
}
