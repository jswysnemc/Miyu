// ============================================================
// 全局应用状态：会话、消息、配置、网关、任务
// 以及检查器抽屉、滚动跟随、流式渲染计数等 UI 状态
// 工具调用已内联到助手消息，不再单独维护 toolEvents
// ============================================================

export const appState = {
  // 会话与消息
  sessions: [],
  activeSessionId: null,
  messages: [],

  // 配置与外围
  config: null,
  gateways: [],
  tasks: [],

  // 输入区
  pendingImageUrl: null,
  pendingImageName: null,
  streaming: false,

  // 检查器抽屉
  inspectorOpen: false,

  // 会话多选
  multiSelectMode: false,
  selectedIds: new Set(),

  // 滚动跟随
  autoScroll: true,

  // 流式渲染：当前消息已渲染的代码围栏数，用于判断代码块是否闭合
  streamFenceCount: 0,
};

/**
 * 获取当前激活的会话对象
 * @returns {Object|null} 当前会话，未选中时返回 null
 */
export function activeSession() {
  return appState.sessions.find((s) => s.id === appState.activeSessionId) || null;
}

/**
 * 设置当前激活会话并持久化到 localStorage
 * @param {string} id - 会话 ID
 * @returns {void}
 */
export function setActiveSession(id) {
  appState.activeSessionId = id;
  localStorage.setItem("miyu.web.activeSessionId", id || "");
}

/**
 * 从 localStorage 恢复上次激活的会话
 * @returns {void}
 */
export function restoreActiveSession() {
  appState.activeSessionId = localStorage.getItem("miyu.web.activeSessionId") || null;
}

/**
 * 设置检查器抽屉开关状态
 * @param {boolean} open - 是否展开
 * @returns {void}
 */
export function setInspectorOpen(open) {
  appState.inspectorOpen = !!open;
}

/**
 * 设置滚动跟随状态
 * @param {boolean} follow - 是否自动滚动到底部
 * @returns {void}
 */
export function setAutoScroll(follow) {
  appState.autoScroll = !!follow;
}
