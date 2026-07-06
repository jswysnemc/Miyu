// ============================================================
// 设置：渠道网关配置卡片（QQ / 微信 iLink）
// 彻底使用定制开关与分段控制器，取代所有原生控件
// ============================================================

import { appState } from "../state.js";
import { field, checkbox, choicePills, setupFormInteractions } from "./form-utils.js";
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
          ${checkbox("gateways.qq.enabled", "开启 QQ 渠道通信桥接", qq.enabled, "启用后智能体将自动建立连接并处理消息群发")}
          ${choicePills(
            "gateways.qq.transport",
            "网络传输协议 (Transport)",
            [
              { label: "WebSocket 实时建立", value: "websocket" },
              { label: "Webhook HTTP 回调", value: "webhook" },
            ],
            qq.transport,
            "选择通信长连接或回调端点模式"
          )}
          ${field("gateways.qq.listen", "本地 Webhook 监听 IP 与端口", qq.listen, "text", "", "127.0.0.1:8766")}
          ${field("gateways.qq.base_url", "OpenAPI 网关基地址", qq.base_url, "text", "full", "https://api.sgroup.qq.com")}
          ${field("gateways.qq.app_id", "平台 AppID", qq.app_id, "text", "", "输入官方申请的 AppID")}
          ${field("gateways.qq.token", "Webhook Token / AppSecret", qq.token, "password", "", "输入鉴权 Token 或密钥")}
          ${field("gateways.qq.client_secret", "AppSecret 密钥", qq.client_secret, "password", "", "输入应用机密密码")}
        </div>
      </div>
      <div class="config-card">
        <div class="card-title">
          <h4>微信企业 iLink / 个人助手通信</h4>
        </div>
        <div class="form-grid">
          ${checkbox("gateways.weixin.enabled", "开启微信通道代理", weixin.enabled, "允许智能体接入微信企业微信或个人对话流")}
          ${choicePills(
            "gateways.weixin.bot_type",
            "机器人适配模式 (Bot Type)",
            [
              { label: "WeChat 企业微信", value: "WeChat" },
              { label: "iLink 个人助手", value: "iLink" },
            ],
            weixin.bot_type,
            "选择对接的微信通道接口协议"
          )}
          ${field("gateways.weixin.base_url", "iLink 协议端点 URL", weixin.base_url, "text", "full", "https://ilink.tencentbot.top")}
          ${field("gateways.weixin.cdn_base_url", "文件媒体资源 CDN 域名", weixin.cdn_base_url, "text", "full", "https://novac2c.cdn.weixin.qq.com")}
          ${field("gateways.weixin.bot_agent", "用户代理名称 (Bot Agent)", weixin.bot_agent, "text", "", "自定义 Bot 标识名称")}
          ${field("gateways.weixin.account", "绑定微信账号标识", weixin.account, "text", "", "输入绑定的微信号或 ID")}
          ${field("gateways.weixin.token", "鉴权 Token 凭据", weixin.token, "password", "", "输入长连接验证 Token")}
        </div>
      </div>
    </div>
  `;

  // 绑定自研 UI 事件（开关切换与选项点击）
  setupFormInteractions(paneEl);
}
