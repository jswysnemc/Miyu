// ============================================================
// /api/chat 流事件分发：将后端 NDJSON 事件分发到聊天视图与工具时间线
// ============================================================

import { streamChat } from "./api.js";
import { appState } from "./state.js";
import { appendAssistantChunk, finishAssistant, showAssistantError } from "./chat/view.js";
import { addToolEvent } from "./inspector/tool-timeline.js";
import { loadSessions } from "./sessions/list.js";
import { setResponding } from "./composer/controls.js";

const modeEl = document.getElementById("modeSelect");
const thinkingEl = document.getElementById("thinkingSelect");

// 当前流式请求的中断控制器
let currentAbort = null;

/**
 * 中断当前流式请求
 * @returns {void}
 */
export function abortCurrentStream() {
  if (currentAbort) {
    currentAbort.abort();
    currentAbort = null;
  }
}

/**
 * 发送当前消息并处理流式响应
 * @param {string} message - 用户消息文本
 * @param {string|null} imageUrl - 附件图片 data URL
 * @returns {Promise<void>}
 */
export async function sendCurrentMessage(message, imageUrl) {
  appState.streaming = true;
  setResponding(true);
  currentAbort = new AbortController();
  try {
    await streamChat(
      {
        session_id: appState.activeSessionId,
        message,
        image_url: imageUrl,
        mode: modeEl.value,
        thinking: thinkingEl.value || null,
      },
      handleStreamEvent,
      currentAbort.signal
    );
    // 1. 流结束后同步会话列表标题与历史
    await loadSessions();
  } catch (error) {
    // 2. 中断不视为错误
    if (error.name !== "AbortError") {
      showAssistantError(error.message);
    }
  } finally {
    currentAbort = null;
    appState.streaming = false;
    setResponding(false);
  }
}

/**
 * 处理单条流式事件
 * @param {Object} event - 事件对象
 * @returns {void}
 */
function handleStreamEvent(event) {
  switch (event.type) {
    case "session":
      // 1. 后端分配的新会话 ID
      appState.activeSessionId = event.session_id;
      break;
    case "chunk":
      // 2. 增量内容或思考
      appendAssistantChunk(event.kind, event.text || "");
      break;
    case "tool_call":
    case "tool_call_progress":
    case "tool_progress":
    case "tool_result":
      // 3. 工具调用事件入时间线
      addToolEvent(event);
      break;
    case "done":
      // 4. 完成，用完整内容重渲染
      finishAssistant(event.content || "", event.reasoning || "");
      break;
    case "error":
      showAssistantError(event.message || "unknown error");
      break;
  }
}
