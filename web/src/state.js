export const appState = {
  sessions: [],
  activeSessionId: null,
  messages: [],
  toolEvents: [],
  config: null,
  gateways: [],
  tasks: [],
  pendingImageUrl: null,
  pendingImageName: null,
  streaming: false,
};

export function activeSession() {
  return appState.sessions.find((session) => session.id === appState.activeSessionId) || null;
}

export function setActiveSession(id) {
  appState.activeSessionId = id;
  localStorage.setItem("miyu.web.activeSessionId", id || "");
}

export function restoreActiveSession() {
  appState.activeSessionId = localStorage.getItem("miyu.web.activeSessionId") || null;
}
