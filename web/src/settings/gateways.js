// ============================================================
// 设置：渠道网关配置表单（QQ / 微信）
// ============================================================

import { appState } from "../state.js";
import { field, checkbox } from "./form-utils.js";

const paneEl = document.getElementById("settingsGateways");

/**
 * 渲染渠道网关配置面板
 * @returns {void}
 */
export function renderGateways() {
  const qq = appState.config.gateways.qq;
  const weixin = appState.config.gateways.weixin;
  paneEl.innerHTML = `
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
