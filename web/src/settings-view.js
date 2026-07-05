import { apiGet, apiPut } from "./api.js";
import { appState } from "./state.js";

const modalEl = document.getElementById("settingsModal");
const statusEl = document.getElementById("settingsStatus");
const providersPane = document.getElementById("settingsProviders");
const toolsPane = document.getElementById("settingsTools");
const gatewaysPane = document.getElementById("settingsGateways");

export async function loadConfig() {
  const data = await apiGet("/api/config");
  appState.config = data.config;
  renderSettings();
}

export function openSettings() {
  renderSettings();
  modalEl.hidden = false;
}

export function closeSettings() {
  modalEl.hidden = true;
}

export async function saveConfig() {
  syncConfigFromForm();
  statusEl.textContent = "保存中";
  const data = await apiPut("/api/config", { config: appState.config });
  appState.config = data.config;
  statusEl.textContent = "已保存";
  renderSettings();
}

export function setupSettingsTabs() {
  document.querySelectorAll(".tab-button").forEach((button) => {
    button.addEventListener("click", () => {
      document.querySelectorAll(".tab-button").forEach((item) => item.classList.remove("active"));
      document.querySelectorAll(".settings-pane").forEach((item) => item.classList.remove("active"));
      button.classList.add("active");
      document.getElementById(`settings${capitalize(button.dataset.tab)}`).classList.add("active");
    });
  });
}

function renderSettings() {
  if (!appState.config) return;
  renderProviders();
  renderTools();
  renderGateways();
}

function renderProviders() {
  const config = appState.config;
  providersPane.innerHTML = `
    <div class="settings-section">
      <h3>当前模型</h3>
      <div class="form-grid">
        <div class="field">
          <label>Active provider</label>
          <select id="activeProviderField">
            ${config.providers.map((provider) => `<option value="${escapeHtml(provider.id)}" ${provider.id === config.active_provider ? "selected" : ""}>${escapeHtml(provider.display_name || provider.id)}</option>`).join("")}
          </select>
        </div>
      </div>
    </div>
    ${config.providers.map((provider, index) => providerForm(provider, index)).join("")}
  `;
}

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

function renderTools() {
  const tools = appState.config.tools;
  toolsPane.innerHTML = `
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

function renderGateways() {
  const qq = appState.config.gateways.qq;
  const weixin = appState.config.gateways.weixin;
  gatewaysPane.innerHTML = `
    <div class="settings-section">
      <h3>QQ 官方机器人</h3>
      <div class="form-grid">
        ${checkbox("gateways.qq.enabled", "启用 QQ", qq.enabled)}
        ${field("gateways.qq.transport", "传输 websocket/webhook", qq.transport)}
        ${field("gateways.qq.listen", "Webhook 监听地址", qq.listen)}
        ${field("gateways.qq.base_url", "OpenAPI Base URL", qq.base_url)}
        ${field("gateways.qq.token", "Token AppID:AppSecret", qq.token, "password")}
        ${field("gateways.qq.app_id", "AppID", qq.app_id)}
        ${field("gateways.qq.client_secret", "AppSecret", qq.client_secret, "password")}
      </div>
    </div>
    <div class="settings-section">
      <h3>微信 iLink</h3>
      <div class="form-grid">
        ${checkbox("gateways.weixin.enabled", "启用微信", weixin.enabled)}
        ${field("gateways.weixin.base_url", "iLink Base URL", weixin.base_url)}
        ${field("gateways.weixin.cdn_base_url", "CDN Base URL", weixin.cdn_base_url)}
        ${field("gateways.weixin.bot_type", "Bot type", weixin.bot_type)}
        ${field("gateways.weixin.token", "Token", weixin.token, "password")}
        ${field("gateways.weixin.account", "已保存账号", weixin.account)}
        ${field("gateways.weixin.bot_agent", "Bot Agent", weixin.bot_agent)}
      </div>
    </div>
  `;
}

function syncConfigFromForm() {
  document.querySelectorAll("[data-config-path]").forEach((input) => {
    const path = input.dataset.configPath;
    let value = input.type === "checkbox" ? input.checked : input.value;
    if (path.endsWith(".models")) {
      value = input.value.split(",").map((item) => item.trim()).filter(Boolean);
    } else if (input.type === "number") {
      value = input.value.includes(".") ? Number.parseFloat(input.value) : Number.parseInt(input.value, 10);
      if (Number.isNaN(value)) value = 0;
    }
    setByPath(appState.config, path, value);
  });
  const activeProvider = document.getElementById("activeProviderField");
  if (activeProvider) {
    appState.config.active_provider = activeProvider.value;
  }
}

function field(path, label, value, type = "text", wide = "") {
  return `
    <div class="field ${wide}">
      <label>${escapeHtml(label)}</label>
      <input data-config-path="${escapeHtml(path)}" type="${type}" value="${escapeHtml(value ?? "")}">
    </div>
  `;
}

function checkbox(path, label, value) {
  return `
    <div class="field">
      <label>${escapeHtml(label)}</label>
      <input data-config-path="${escapeHtml(path)}" type="checkbox" ${value ? "checked" : ""}>
    </div>
  `;
}

function setByPath(target, path, value) {
  const parts = path.split(".");
  let cursor = target;
  for (const part of parts.slice(0, -1)) {
    cursor = cursor[part];
  }
  cursor[parts.at(-1)] = value;
}

function capitalize(value) {
  return value.slice(0, 1).toUpperCase() + value.slice(1);
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
