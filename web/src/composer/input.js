// ============================================================
// 输入框：textarea 自适应、快捷键、提交
// ============================================================

import { appState } from "../state.js";
import { appendUserMessage } from "../chat/view.js";
import { sendCurrentMessage } from "../stream.js";
import { clearAttachment, getAttachment } from "./attachments.js";

const formEl = document.getElementById("composer");
const inputEl = document.getElementById("messageInput");

/**
 * 初始化输入框事件
 * @returns {void}
 */
export function setupComposer() {
  inputEl.addEventListener("input", resizeInput);
  inputEl.addEventListener("keydown", (event) => {
    // 1. Enter 发送，Shift+Enter 换行
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      formEl.requestSubmit();
    }
  });
  formEl.addEventListener("submit", async (event) => {
    event.preventDefault();
    const message = inputEl.value.trim();
    const imageUrl = getAttachment();
    // 1. 空内容或正在流式中禁止提交
    if ((!message && !imageUrl) || appState.streaming) return;
    appendUserMessage(message, imageUrl);
    inputEl.value = "";
    clearAttachment();
    resizeInput();
    await sendCurrentMessage(message, imageUrl);
  });
}

/**
 * 自适应 textarea 高度
 * @returns {void}
 */
function resizeInput() {
  inputEl.style.height = "auto";
  inputEl.style.height = `${Math.min(inputEl.scrollHeight, 160)}px`;
}
