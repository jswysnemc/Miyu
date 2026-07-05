// ============================================================
// 内联工具调用管理：在助手消息流中挂载工具卡片
// 处理 tool_call / tool_call_progress / tool_progress / tool_result 事件
// 工具顺序执行，同一时刻仅一个活跃卡片
// ============================================================

import {
  getToolIcon,
  summarizeTool,
  renderToolBody,
} from "../tools/registry.js";
import {
  createToolCard,
  setCardStatus,
  parseArgs,
  clipSummary,
} from "../tools/common.js";

// 当前助手消息的工具区引用
let currentToolHost = null;

/**
 * 在助手消息块上创建工具区容器
 * 容器位于消息主体之后、操作栏之前
 * @param {HTMLElement} block - 助手消息块元素
 * @returns {HTMLElement} 工具区容器
 */
export function createToolHost(block) {
  const host = document.createElement("div");
  host.className = "tool-host";
  const actionsEl = block.querySelector(".message-actions");
  if (actionsEl) {
    block.insertBefore(host, actionsEl);
  } else {
    block.appendChild(host);
  }
  currentToolHost = { hostEl: host, currentCard: null };
  return host;
}

/**
 * 重置工具区引用，用于新消息开始或切换会话
 * @returns {void}
 */
export function resetToolHost() {
  currentToolHost = null;
}

/**
 * 确保存在一个活跃的工具卡片，若不存在则创建
 * @param {string} name - 工具名
 * @returns {Object} 当前卡片上下文
 */
function ensureActiveCard(name) {
  if (!currentToolHost) return null;
  if (currentToolHost.currentCard && currentToolHost.currentCard.name === name) {
    return currentToolHost.currentCard;
  }
  // 1. 若已有不同工具的活跃卡片，先收尾为未知状态
  if (currentToolHost.currentCard) {
    finalizePendingCard(currentToolHost.currentCard);
  }
  // 2. 创建新卡片
  const icon = getToolIcon(name);
  const card = createToolCard({ name, iconSvg: icon, summary: "", running: true });
  currentToolHost.hostEl.appendChild(card.block);
  currentToolHost.currentCard = {
    name,
    arguments: "",
    block: card.block,
    summaryEl: card.summaryEl,
    bodyEl: card.bodyEl,
    statusEl: card.statusEl,
  };
  return currentToolHost.currentCard;
}

/**
 * 处理 tool_call 事件：参数完整到达，更新摘要
 * @param {string} name - 工具名
 * @param {string} argumentsRaw - 完整参数 JSON
 * @returns {void}
 */
export function addToolCall(name, argumentsRaw) {
  const card = ensureActiveCard(name);
  if (!card) return;
  card.arguments = argumentsRaw || "";
  // 1. 用完整参数重算摘要
  const summary = summarizeTool(name, argumentsRaw);
  if (summary) card.summaryEl.textContent = summary;
}

/**
 * 处理 tool_call_progress 事件：参数流式预览
 * @param {Object} event - 事件 { name, arguments_preview, index }
 * @returns {void}
 */
export function addToolCallProgress(event) {
  const name = event.name || inferName();
  const card = ensureActiveCard(name);
  if (!card) return;
  // 1. 参数尚未完整时用预览显示临时摘要
  if (event.arguments_preview && !card.arguments) {
    card.summaryEl.textContent = clipSummary(event.arguments_preview, 80);
  }
}

/**
 * 处理 tool_progress 事件：执行进度消息
 * @param {Object} event - 事件 { name, message }
 * @returns {void}
 */
export function addToolProgress(event) {
  const name = event.name || inferName();
  const card = ensureActiveCard(name);
  if (!card) return;
  if (event.message) {
    const line = ensureProgressLine(card.bodyEl);
    line.textContent = event.message;
  }
}

/**
 * 处理 tool_result 事件：工具完成，渲染输出并折叠
 * @param {string} name - 工具名
 * @param {boolean} ok - 是否成功
 * @param {string} output - 输出文本
 * @returns {void}
 */
export function finishToolResult(name, ok, output) {
  if (!currentToolHost) return;
  const card = currentToolHost.currentCard;
  if (!card) return;
  // 1. 设置状态并折叠
  setCardStatus(card.block, card.statusEl, ok ? "ok" : "error", true);
  // 2. 渲染输出区
  const ctx = {
    name: card.name,
    args: parseArgs(card.arguments),
    output: output || "",
    ok,
    bodyEl: card.bodyEl,
    summaryEl: card.summaryEl,
  };
  renderToolBody(card.name, ctx);
  // 3. 用完整参数重算摘要（若 addToolCall 已设置则保持）
  const summary = summarizeTool(card.name, card.arguments);
  if (summary) card.summaryEl.textContent = summary;
  // 4. 清除活跃卡片引用
  currentToolHost.currentCard = null;
}

/**
 * 推断当前工具名（progress 事件可能未带 name）
 * @returns {string}
 */
function inferName() {
  return currentToolHost && currentToolHost.currentCard
    ? currentToolHost.currentCard.name
    : "tool";
}

/**
 * 在输出区确保存在一行进度提示
 * @param {HTMLElement} bodyEl - 输出区元素
 * @returns {HTMLElement} 进度行元素
 */
function ensureProgressLine(bodyEl) {
  let line = bodyEl.querySelector(".tool-progress-line");
  if (!line) {
    line = document.createElement("div");
    line.className = "tool-progress-line";
    bodyEl.appendChild(line);
  }
  return line;
}

/**
 * 收尾未完成的卡片（切换工具时调用）
 * @param {Object} card - 卡片上下文
 * @returns {void}
 */
function finalizePendingCard(card) {
  setCardStatus(card.block, card.statusEl, "ok", true);
  // 1. 无结果输出时显示空状态
  if (!card.bodyEl.children.length) {
    card.bodyEl.innerHTML = '<div class="tool-output-empty">无输出</div>';
  }
}
