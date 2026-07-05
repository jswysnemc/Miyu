// ============================================================
// 聊天视图：消息列表渲染、流式追加、滚动控制、回到底部按钮
// ============================================================

import { appState, activeSession, setAutoScroll } from "../state.js";
import { createMessageNode, buildThoughtProcess } from "./message-node.js";
import { createStreamRenderer } from "../markdown/stream-renderer.js";
import { renderFull } from "../markdown/renderer.js";
import {
  createToolHost,
  resetToolHost,
} from "./tool-inline.js";

const chatContainer = document.getElementById("chatContainer");
const chatInner = document.getElementById("chatInner");
const welcomeEl = document.getElementById("welcome");
const titleEl = document.getElementById("activeSessionTitle");
const scrollDownBtn = document.getElementById("scrollDownBtn");

// 当前流式渲染上下文
let currentStream = null;

/**
 * 初始化滚动监听与回到底部按钮
 * @returns {void}
 */
export function setupScroll() {
  chatContainer.addEventListener("scroll", () => {
    const atBottom =
      chatContainer.scrollHeight - chatContainer.scrollTop - chatContainer.clientHeight < 40;
    setAutoScroll(atBottom);
    scrollDownBtn.classList.toggle("show", !atBottom && appState.messages.length > 0);
  });
  scrollDownBtn.addEventListener("click", () => {
    setAutoScroll(true);
    scrollToBottom();
  });
}

/**
 * 滚动到底部
 * @returns {void}
 */
function scrollToBottom() {
  chatContainer.scrollTop = chatContainer.scrollHeight;
}

/**
 * 重新渲染当前会话的全部消息
 * @returns {void}
 */
export function renderChat() {
  const session = activeSession();
  titleEl.textContent = session ? session.title : "新会话";
  // 1. 清空除 welcome 外的节点
  Array.from(chatInner.children).forEach((child) => {
    if (child !== welcomeEl) child.remove();
  });
  welcomeEl.hidden = appState.messages.length > 0;
  // 2. 逐条渲染
  appState.messages.forEach((message, index) => {
    const node = createMessageNode(message, index);
    chatInner.appendChild(node.block);
  });
  // 3. 重置工具区引用，历史消息不挂载工具卡片
  resetToolHost();
  currentStream = null;
  if (appState.autoScroll) scrollToBottom();
}

/**
 * 追加用户消息
 * @param {string} content - 文本内容
 * @param {string|null} imageUrl - 图片 URL
 * @returns {void}
 */
export function appendUserMessage(content, imageUrl) {
  appState.messages.push({
    role: "user",
    content,
    imageUrl: imageUrl || null,
    done: true,
  });
  renderChat();
}

/**
 * 确保存在一个未完成的助手消息用于接收流式 chunk
 * @returns {{message: Object, block: HTMLElement, thoughtEl: HTMLElement|null, thoughtProcessEl: HTMLElement|null, streamRenderer: Object}}
 */
export function ensureAssistantMessage() {
  const last = appState.messages[appState.messages.length - 1];
  if (!last || last.role !== "assistant" || last.done) {
    appState.messages.push({
      role: "assistant",
      content: "",
      reasoning: "",
      done: false,
    });
    const index = appState.messages.length - 1;
    const node = createMessageNode(appState.messages[index], index);
    chatInner.appendChild(node.block);
    welcomeEl.hidden = true;
    // 1. 为新助手消息挂载工具调用区
    createToolHost(node.block);
    currentStream = {
      message: appState.messages[index],
      block: node.block,
      thoughtEl: node.thoughtEl,
      thoughtProcessEl: node.thoughtProcessEl,
      streamRenderer: createStreamRenderer(node.contentEl),
    };
  }
  return currentStream;
}

/**
 * 追加助手流式增量
 * @param {string} kind - chunk 类型 reasoning 或 content
 * @param {string} text - 增量文本
 * @returns {void}
 */
export function appendAssistantChunk(kind, text) {
  const stream = ensureAssistantMessage();
  if (kind === "reasoning") {
    // 1. 首次 reasoning 时按需创建思考过程区域
    if (!stream.thoughtProcessEl) {
      const result = buildThoughtProcess({ done: false });
      stream.thoughtProcessEl = result.thoughtProcessEl;
      stream.thoughtEl = result.thoughtEl;
      stream.block.insertBefore(result.thoughtProcessEl, stream.block.firstChild);
      stream.streamRenderer.attachThought(result.thoughtEl);
    }
    stream.streamRenderer.writeThought(text);
  } else {
    stream.streamRenderer.writeContent(text);
  }
  if (appState.autoScroll) scrollToBottom();
}

/**
 * 完成当前助手消息，用完整内容重渲染
 * @param {string} content - 完整正文
 * @param {string} reasoning - 完整思考
 * @returns {void}
 */
export function finishAssistant(content, reasoning) {
  const stream = ensureAssistantMessage();
  stream.message.done = true;
  if (content) stream.message.content = content;
  if (reasoning) stream.message.reasoning = reasoning;
  // 1. 若有 reasoning 但尚未创建思考过程，补建
  if (reasoning && !stream.thoughtProcessEl) {
    const result = buildThoughtProcess({ done: false });
    stream.thoughtProcessEl = result.thoughtProcessEl;
    stream.thoughtEl = result.thoughtEl;
    stream.block.insertBefore(result.thoughtProcessEl, stream.block.firstChild);
    stream.streamRenderer.attachThought(result.thoughtEl);
  }
  // 2. 切换思考过程为完成态
  if (stream.thoughtProcessEl) {
    stream.thoughtProcessEl.classList.remove("thinking");
    const bar = stream.thoughtProcessEl.querySelector(".thought-bar");
    if (bar) {
      bar.innerHTML =
        '<span class="thought-label">思考过程</span><svg class="thought-arrow" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="6 9 12 15 18 9"></polyline></svg>';
    }
  }
  // 3. 用完整文本重渲染
  stream.streamRenderer.finalize(content || "", reasoning || "");
  if (appState.autoScroll) scrollToBottom();
}

/**
 * 在当前助手消息中追加错误提示
 * @param {string} message - 错误文本
 * @returns {void}
 */
export function showAssistantError(message) {
  const stream = ensureAssistantMessage();
  stream.message.content += `\n\n错误: ${message}`;
  stream.message.done = true;
  // 1. 移除思考中态
  if (stream.thoughtProcessEl) {
    stream.thoughtProcessEl.classList.remove("thinking");
    stream.thoughtProcessEl.remove();
    stream.thoughtProcessEl = null;
    stream.thoughtEl = null;
  }
  // 2. 渲染错误内容
  renderFull(stream.block.querySelector(".message-content"), stream.message.content);
  if (appState.autoScroll) scrollToBottom();
}
