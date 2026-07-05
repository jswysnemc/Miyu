import { apiGet, apiPost } from "./api.js";
import { appState, restoreActiveSession, setActiveSession } from "./state.js";
import { renderChat } from "./chat-view.js";

const listEl = document.getElementById("sessionList");

export async function loadSessions() {
  restoreActiveSession();
  const data = await apiGet("/api/sessions");
  appState.sessions = data.sessions || [];
  if (appState.sessions.length === 0) {
    const created = await apiPost("/api/sessions");
    appState.sessions = [created.session];
  }
  if (!appState.sessions.some((session) => session.id === appState.activeSessionId)) {
    setActiveSession(appState.sessions[0].id);
  }
  await loadActiveSessionHistory();
  renderSessions();
}

export async function createSession() {
  const data = await apiPost("/api/sessions");
  appState.sessions.unshift(data.session);
  setActiveSession(data.session.id);
  appState.messages = [];
  renderSessions();
  renderChat();
}

export async function selectSession(id) {
  setActiveSession(id);
  await loadActiveSessionHistory();
  renderSessions();
  renderChat();
}

export async function loadActiveSessionHistory() {
  if (!appState.activeSessionId) return;
  const data = await apiGet(`/api/sessions/${encodeURIComponent(appState.activeSessionId)}`);
  appState.messages = (data.messages || []).map((entry) => ({
    role: entry.role,
    content: entry.content,
    reasoning: entry.reasoning || "",
    done: true,
  }));
}

export function renderSessions() {
  listEl.innerHTML = appState.sessions.map((session) => `
    <button class="session-item ${session.id === appState.activeSessionId ? "active" : ""}" data-session-id="${session.id}">
      <span class="session-title">${escapeHtml(session.title)}</span>
    </button>
  `).join("");
  listEl.querySelectorAll("[data-session-id]").forEach((button) => {
    button.addEventListener("click", () => selectSession(button.dataset.sessionId));
  });
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}
