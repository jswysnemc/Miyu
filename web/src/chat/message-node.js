// ============================================================
// 单条消息 DOM 构造：用户/助手消息、思考过程、操作栏
// ============================================================

import { renderFull, renderPlainText } from "../markdown/renderer.js";
import { createMessageActions } from "./actions.js";

/**
 * 构造一条消息的 DOM 节点
 * @param {Object} message - 消息对象 {role, content, reasoning, imageUrl, done}
 * @param {number} index - 消息在列表中的索引
 * @returns {{block: HTMLElement, contentEl: HTMLElement, thoughtEl: HTMLElement|null, thoughtProcessEl: HTMLElement|null}}
 */
export function createMessageNode(message, index) {
  const role = message.role || "assistant";
  const block = document.createElement("div");
  block.className = `message-block ${role}-block`;

  // 1. 助手消息的思考过程（仅已完成且有 reasoning 时在此创建；流式时由 view 按需创建）
  let thoughtEl = null;
  let thoughtProcessEl = null;
  if (role === "assistant" && message.reasoning && message.done) {
    const result = buildThoughtProcess(message);
    thoughtProcessEl = result.thoughtProcessEl;
    thoughtEl = result.thoughtEl;
    block.appendChild(thoughtProcessEl);
  }

  // 2. 消息主体
  const messageDiv = document.createElement("div");
  messageDiv.className = `message ${role}`;
  const contentEl = document.createElement("div");
  contentEl.className = "message-content";
  contentEl.setAttribute("data-index", String(index));

  if (message.done) {
    // 1. 已完成消息：立即渲染内容
    if (role === "user") {
      renderPlainText(contentEl, message.content || "");
    } else {
      renderFull(contentEl, message.content || "");
    }
  } else {
    // 2. 流式中：内容由 stream-renderer 写入，这里仅占位
    contentEl.setAttribute("data-raw", "");
  }
  messageDiv.appendChild(contentEl);

  // 3. 用户图片附件
  if (message.imageUrl) {
    const urls = Array.isArray(message.imageUrl) ? message.imageUrl : [message.imageUrl];
    for (const url of urls) {
      const img = document.createElement("img");
      img.src = url;
      img.className = "user-image";
      img.alt = "";
      img.addEventListener("click", () => {
        window.dispatchEvent(new CustomEvent("miyu:lightbox", { detail: { src: url } }));
      });
      messageDiv.appendChild(img);
    }
  }
  block.appendChild(messageDiv);

  // 4. 操作栏
  const actions = createMessageActions(role, contentEl);
  block.appendChild(actions);

  // 5. 渲染后绑定正文内图片的点击放大
  if (message.done) {
    bindImageLightbox(contentEl);
  }

  return { block, contentEl, thoughtEl, thoughtProcessEl };
}

/**
 * 构造思考过程区域
 * @param {Object} message - 消息对象
 * @returns {{thoughtProcessEl: HTMLElement, thoughtEl: HTMLElement}}
 */
export function buildThoughtProcess(message) {
  const thoughtProcessEl = document.createElement("div");
  thoughtProcessEl.className = "thought-process";
  if (!message.done) {
    thoughtProcessEl.classList.add("thinking");
  }

  const bar = document.createElement("div");
  bar.className = "thought-bar";
  if (!message.done) {
    // 1. 流式中显示跳动点动效
    bar.innerHTML =
      '<span class="thinking-dots"><span></span><span></span><span></span></span><span class="thought-label">思考中</span>';
  } else {
    // 2. 完成后显示折叠条
    bar.innerHTML =
      `<span class="thought-label">思考过程</span><svg class="thought-arrow" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="6 9 12 15 18 9"></polyline></svg>`;
  }

  const thoughtEl = document.createElement("div");
  thoughtEl.className = "thought-content";
  if (message.done && message.reasoning) {
    renderFull(thoughtEl, message.reasoning);
    bindImageLightbox(thoughtEl);
  } else {
    thoughtEl.setAttribute("data-raw", "");
  }

  // 3. 点击 bar 切换折叠
  bar.addEventListener("click", () => {
    thoughtProcessEl.classList.toggle("collapsed");
  });

  thoughtProcessEl.appendChild(bar);
  thoughtProcessEl.appendChild(thoughtEl);
  return { thoughtProcessEl, thoughtEl };
}

/**
 * 为容器内的图片绑定点击放大
 * @param {HTMLElement} container - DOM 容器
 * @returns {void}
 */
function bindImageLightbox(container) {
  const imgs = container.querySelectorAll("img");
  imgs.forEach((img) => {
    if (img.dataset.lightboxBound) return;
    img.dataset.lightboxBound = "1";
    img.addEventListener("click", () => {
      window.dispatchEvent(
        new CustomEvent("miyu:lightbox", { detail: { src: img.src } })
      );
    });
  });
}
