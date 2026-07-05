// ============================================================
// 设置：模型 provider 表单渲染
// ============================================================

import { appState } from "../state.js";
import { field, escapeHtml } from "./form-utils.js";

const paneEl = document.getElementById("settingsProviders");

/**
 * 渲染模型 provider 配置面板
 * @returns {void}
 */
export function renderProviders() {
  const config = appState.config;
  paneEl.innerHTML = `
    <div class="settings-section">
      <h3>当前模型</h3>
      <div class="form-grid">
        <div class="field">
          <label>Active provider</label>
          <select id="activeProviderField">
            ${config.providers
              .map(
                (provider) =>
                  `<option value="${escapeHtml(provider.id)}" ${provider.id === config.active_provider ? "selected" : ""}>${escapeHtml(provider.display_name || provider.id)}</option>`
              )
              .join("")}
          </select>
        </div>
      </div>
    </div>
    ${config.providers.map((provider, index) => providerForm(provider, index)).join("")}
  `;
}

/**
 * 渲染单个 provider 的表单
 * @param {Object} provider - provider 配置
 * @param {number} index - 在 providers 数组中的索引
 * @returns {string} HTML 字符串
 */
function providerForm(provider, index) {
  return `
    <div class="settings-section" data-provider-index="${index}">
      <h3>${escapeHtml(provider.display_name || provider.id)}</h3>
      <div class="form-grid">
        ${field(`providers.${index}.display_name`, "名称", provider.display_name)}
        ${field(`providers.${index}.base_url`, "Base URL", provider.base_url)}
        ${field(`providers.${index}.api_key`, "API Key", provider.api_key || "", "password")}
        ${field(`providers.${index}.default_model`, "默认模型", provider.default_model)}
        ${field(`providers.${index}.protocol`, "协议", provider.protocol || "auto")}
        ${field(`providers.${index}.thinking_level`, "思考等级", provider.thinking_level || "auto")}
        ${field(`providers.${index}.thinking_format`, "思考格式", provider.thinking_format || "auto")}
        ${field(`providers.${index}.timeout_seconds`, "超时秒数", provider.timeout_seconds, "number")}
        ${field(`providers.${index}.temperature`, "Temperature", provider.temperature, "number")}
        ${field(`providers.${index}.models`, "模型列表，逗号分隔", (provider.models || []).join(", "), "text", "full")}
      </div>
    </div>
  `;
}
