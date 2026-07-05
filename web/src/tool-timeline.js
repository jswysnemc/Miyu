import { appState } from "./state.js";

const timelineEl = document.getElementById("toolTimeline");

export function addToolEvent(event) {
  appState.toolEvents.unshift({
    ...event,
    time: new Date().toLocaleTimeString(),
  });
  appState.toolEvents = appState.toolEvents.slice(0, 80);
  renderToolTimeline();
}

export function clearToolTimeline() {
  appState.toolEvents = [];
  renderToolTimeline();
}

export function renderToolTimeline() {
  if (appState.toolEvents.length === 0) {
    timelineEl.innerHTML = '<div class="empty-note">暂无工具调用</div>';
    return;
  }
  timelineEl.innerHTML = appState.toolEvents.map(renderTool).join("");
}

function renderTool(event) {
  const status = event.type === "tool_result" ? (event.ok ? "ok" : "error") : "running";
  const name = event.name || `tool-${event.index ?? ""}`;
  const body = event.output || event.message || event.arguments_preview || event.arguments || "";
  return `
    <div class="tool-item">
      <div class="tool-head">
        <span class="tool-name">${escapeHtml(name)}</span>
        <span class="status-pill ${status}">${escapeHtml(event.type)}</span>
      </div>
      <div class="tool-body">${escapeHtml(body).slice(0, 1200)}</div>
    </div>
  `;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
