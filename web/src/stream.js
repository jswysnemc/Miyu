import { streamChat } from "./api.js";
import { appState } from "./state.js";
import { appendAssistantChunk, finishAssistant, showAssistantError } from "./chat-view.js";
import { addToolEvent } from "./tool-timeline.js";
import { loadActiveSessionHistory, loadSessions, renderSessions } from "./sessions-view.js";

const modeEl = document.getElementById("modeSelect");
const thinkingEl = document.getElementById("thinkingSelect");
const sendBtn = document.getElementById("sendBtn");
const connectionStateEl = document.getElementById("connectionState");

export async function sendCurrentMessage(message, imageUrl) {
  appState.streaming = true;
  sendBtn.disabled = true;
  connectionStateEl.textContent = "生成中";
  try {
    await streamChat({
      session_id: appState.activeSessionId,
      message,
      image_url: imageUrl,
      mode: modeEl.value,
      thinking: thinkingEl.value || null,
    }, handleStreamEvent);
    await loadSessions();
    await loadActiveSessionHistory();
    renderSessions();
  } catch (error) {
    showAssistantError(error.message);
  } finally {
    appState.streaming = false;
    sendBtn.disabled = false;
    connectionStateEl.textContent = "本地服务";
  }
}

function handleStreamEvent(event) {
  switch (event.type) {
    case "session":
      appState.activeSessionId = event.session_id;
      break;
    case "chunk":
      appendAssistantChunk(event.kind, event.text || "");
      break;
    case "tool_call":
    case "tool_call_progress":
    case "tool_progress":
    case "tool_result":
      addToolEvent(event);
      break;
    case "done":
      finishAssistant(event.content || "", event.reasoning || "");
      break;
    case "error":
      showAssistantError(event.message || "unknown error");
      break;
  }
}
