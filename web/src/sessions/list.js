// ============================================================
// 会话列表：加载、创建、切换、历史、渲染
// ============================================================

import { apiGet, apiPost, apiDelete } from "../api.js";
import { appState, restoreActiveSession, setActiveSession } from "../state.js";
import { renderChat } from "../chat/view.js";

const listEl = document.getElementById("sessionList");

/**
 * 加载会话列表，若无则自动创建一个
 * @returns {Promise<void>}
 */
export async function loadSessions() {
  restoreActiveSession();
  const data = await apiGet("/api/sessions");
  appState.sessions = data.sessions || [];
  // 1. 无会话时自动创建
  if (appState.sessions.length === 0) {
    const created = await apiPost("/api/sessions");
    appState.sessions = [created.session];
  }
  // 2. 恢复或选首个
  if (!appState.sessions.some((s) => s.id === appState.activeSessionId)) {
    setActiveSession(appState.sessions[0].id);
  }
  await loadActiveSessionHistory();
  renderSessions();
}

/**
 * 创建新会话
 * @returns {Promise<void>}
 */
export async function createSession() {
  const data = await apiPost("/api/sessions");
  appState.sessions.unshift(data.session);
  setActiveSession(data.session.id);
  appState.messages = [];
  renderSessions();
  renderChat();
}

/**
 * 切换到指定会话
 * @param {string} id - 会话 ID
 * @returns {Promise<void>}
 */
export async function selectSession(id) {
  if (appState.multiSelectMode) return;
  setActiveSession(id);
  await loadActiveSessionHistory();
  renderSessions();
  renderChat();
}

/**
 * 加载当前激活会话的历史消息
 * @returns {Promise<void>}
 */
export async function loadActiveSessionHistory() {
  if (!appState.activeSessionId) return;
  const data = await apiGet(
    `/api/sessions/${encodeURIComponent(appState.activeSessionId)}`
  );
  appState.messages = (data.messages || []).map((entry) => ({
    role: entry.role,
    content: entry.content,
    reasoning: entry.reasoning || "",
    done: true,
  }));
}

/**
 * 删除指定会话
 * @param {string} id - 会话 ID
 * @returns {Promise<void>}
 */
export async function deleteSession(id) {
  await apiDelete(`/api/sessions/${encodeURIComponent(id)}`);
  appState.sessions = appState.sessions.filter((s) => s.id !== id);
  appState.selectedIds.delete(id);
  // 1. 删除的是当前会话则切换到首个
  if (appState.activeSessionId === id) {
    if (appState.sessions.length > 0) {
      setActiveSession(appState.sessions[0].id);
      await loadActiveSessionHistory();
    } else {
      setActiveSession(null);
      appState.messages = [];
    }
    renderChat();
  }
  renderSessions();
}

/**
 * 渲染会话列表
 * @param {string} [keyword=""] - 搜索关键词，空则显示全部
 * @returns {void}
 */
export function renderSessions(keyword = "") {
  const lower = keyword.trim().toLowerCase();
  const sessions = lower
    ? appState.sessions.filter((s) => (s.title || "").toLowerCase().includes(lower))
    : appState.sessions;

  if (sessions.length === 0) {
    listEl.innerHTML = `<div class="session-empty">${lower ? "未匹配到会话" : "暂无会话"}</div>`;
    return;
  }

  listEl.innerHTML = "";
  listEl.classList.toggle("multi-select-mode", appState.multiSelectMode);

  for (const session of sessions) {
    const item = document.createElement("button");
    item.className = "session-item";
    if (session.id === appState.activeSessionId && !appState.multiSelectMode) {
      item.classList.add("active");
    }
    if (appState.selectedIds.has(session.id)) {
      item.classList.add("selected");
    }

    // 1. 多选 checkbox
    const checkbox = document.createElement("span");
    checkbox.className = "session-checkbox";
    if (appState.selectedIds.has(session.id)) checkbox.classList.add("checked");

    // 2. 标题
    const title = document.createElement("span");
    title.className = "session-title";
    title.textContent = session.title || "新会话";

    item.append(checkbox, title);

    // 3. 非多选模式下显示删除按钮
    if (!appState.multiSelectMode) {
      const iconsWrap = document.createElement("span");
      iconsWrap.className = "session-icons";
      const delBtn = document.createElement("button");
      delBtn.className = "icon-action";
      delBtn.title = "删除";
      delBtn.innerHTML = '<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>';
      delBtn.addEventListener("click", (event) => {
        event.stopPropagation();
        deleteSession(session.id);
      });
      iconsWrap.appendChild(delBtn);
      item.appendChild(iconsWrap);
    }

    item.addEventListener("click", () => {
      if (appState.multiSelectMode) {
        if (appState.selectedIds.has(session.id)) {
          appState.selectedIds.delete(session.id);
        } else {
          appState.selectedIds.add(session.id);
        }
        renderSessions(keyword);
        updateMultiSelectCount();
      } else {
        selectSession(session.id);
      }
    });

    listEl.appendChild(item);
  }
}

/**
 * 更新多选计数显示
 * @returns {void}
 */
export function updateMultiSelectCount() {
  const countEl = document.getElementById("msCount");
  if (countEl) countEl.textContent = `已选 ${appState.selectedIds.size} 项`;
}
