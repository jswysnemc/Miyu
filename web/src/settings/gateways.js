// ============================================================
// 设置：渠道网关配置卡片（QQ / 微信 iLink）
// 采用双列连接卡片形式排版
// ============================================================

import { appState } from "../state.js";
import { field, checkbox } from "./form-utils.js";
import { icon } from "../icons.js";

const paneEl = document.getElementById("settingsGateways");

/**
 * 渲染渠道网关配置面板
 * @returns {void}
 */
export function renderGateways() {
  if (!paneEl || !appState.config) return;
  const qq = appState.config.gateways.qq;
  const weixin = appState.config.gateways.weixin;

  paneEl.innerHTML = `
    <div class="settings-top-header">
      <span class="header-icon">${icon("network")}</span>
      <div>
        <h3>通信渠道网关与第三方网络桥接 (Gateways)</h3>
        <p>设定将智能体以自动化机器人形态介入 QQ 或微信 iLink 的网络监听端点与密钥。</p>
      </div>
    </div>
    <div class="settings-cards-wrap two-col">
      <div class="config-card">
        <div class="card-title">
          <h4>QQ 官方开放平台机器人</h4>
        </div>
        <div class="form-grid">
          ${checkbox("gateways.qq.enabled", "开启 QQ 渠道通信桥接", qq.enabled)}
          ${field("gateways.qq.transport", "网络传输协议 (websocket / webhook)", qq.transport)}
          ${field("gateways.qq.listen", "本地 Webhook 监听 IP 与端口", qq.listen)}
          ${field("gateways.qq.base_url", "OpenAPI 网关基地址", qq.base_url)}
          ${field("gateways.qq.app_id", "平台 AppID", qq.app_id)}
          ${field("gateways.qq.token", "Webhook Token 或 AppID:AppSecret", qq.token, "password")}
          ${field("gateways.qq.client_secret", "AppSecret 密钥", qq.client_secret, "password")}
        </div>
      </div>
      <div class="config-card">
        <div class="card-title">
          <h4>微信企业 iLink / 个人助手通信</h4>
        </div>
        <div class="form-grid">
          ${checkbox("gateways.weixin.enabled", "开启微信通道代理", weixin.enabled)}
          ${field("gateways.weixin.bot_type", "机器人适配模式 (Bot Type)", weixin.bot_type)}
          ${field("gateways.weixin.base_url", "iLink 协议端点 URL", weixin.base_url)}
          ${field("gateways.weixin.cdn_base_url", "文件媒体资源 CDN 域名", weixin.cdn_base_url)}
          ${field("gateways.weixin.bot_agent", "用户代理名称 (Bot Agent)", weixin.bot_agent)}
          ${field("gateways.weixin.account", "绑定微信账号标识", weixin.account)}
          ${field("gateways.weixin.token", "鉴权 Token 凭据", weixin.token, "password")}
        </div>
      </div>
    </div>
  `;
}
