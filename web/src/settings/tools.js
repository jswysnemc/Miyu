// ============================================================
// 设置：工具运行配置表单
// ============================================================

import { appState } from "../state.js";
import { field, checkbox } from "./form-utils.js";

const paneEl = document.getElementById("settingsTools");

/**
 * 渲染工具配置面板
 * @returns {void}
 */
export function renderTools() {
  const tools = appState.config.tools;
  paneEl.innerHTML = `
    <div class="settings-section">
      <h3>工具运行</h3>
      <div class="form-grid">
        ${checkbox("tools.enabled", "启用工具", tools.enabled)}
        ${checkbox("tools.progressive_loading_enabled", "渐进式加载", tools.progressive_loading_enabled)}
        ${checkbox("tools.background_commands_enabled", "后台命令", tools.background_commands_enabled)}
        ${field("tools.max_rounds", "最大工具轮数，0 为无限", tools.max_rounds, "number")}
        ${field("tools.background_command_timeout_seconds", "后台默认超时秒数，0 为无限", tools.background_command_timeout_seconds, "number")}
        ${field("tools.background_command_log_max_bytes", "后台日志最大字节", tools.background_command_log_max_bytes, "number")}
      </div>
    </div>
  `;
}
