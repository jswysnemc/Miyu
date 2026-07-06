// ============================================================
// 设置：模型 Provider 概览列表与卡片网格渲染
// 摒弃无脑列表堆砌，采用卡片网格呈现，点击卡片展开侧滑编辑器
// ============================================================

import { appState } from "../state.js";
import { escapeHtml } from "./form-utils.js";
import { openProviderEditor } from "./provider-editor.js";
import { createCustomSelect } from "../components/custom-select.js";
import { icon } from "../icons.js";

const paneEl = document.getElementById("settingsProviders");

/**
 * 渲染模型提供商概览卡片页
 * @returns {void}
 */
export function renderProviders() {
  if (!paneEl || !appState.config) return;
  const config = appState.config;

  paneEl.innerHTML = "";

  // 1. 顶部全局 Provider 切换区
  const topBar = document.createElement("div");
  topBar.className = "provider-top-bar";
  topBar.innerHTML = `
    <div class="top-bar-left">
      <span class="bar-icon">${icon("model")}</span>
      <div>
        <h3>有效推理控制台与模型提供商 (Providers)</h3>
        <p>选择主激活服务商或对各通道推理参数进行独立深度微调。</p>
      </div>
    </div>
    <div class="top-bar-right" id="activeSelectContainer">
      <span class="label-text">当前活跃驱动:</span>
    </div>
  `;

  // 2. 使用定制无障碍下拉组件设置当前主服务商
  const activeSelect = createCustomSelect({
    options: config.providers.map((p) => ({
      value: p.id,
      label: p.display_name || p.id,
    })),
    value: config.active_provider || (config.providers[0] ? config.providers[0].id : ""),
    onChange: (val) => {
      config.active_provider = val;
      renderProviders();
    },
  });
  topBar.querySelector("#activeSelectContainer").appendChild(activeSelect);
  paneEl.appendChild(topBar);

  // 3. 服务商卡片阵列
  const gridEl = document.createElement("div");
  gridEl.className = "provider-cards-grid";

  config.providers.forEach((provider, index) => {
    const card = document.createElement("div");
    const isActive = provider.id === config.active_provider;
    card.className = `provider-card ${isActive ? "active-card" : ""}`;

    const modelsList = provider.models || [];
    const modelsPreview = modelsList.slice(0, 4).map((m) => `<span class="preview-chip">${escapeHtml(m)}</span>`).join("");
    const moreCount = modelsList.length > 4 ? `<span class="preview-more">+${modelsList.length - 4}</span>` : "";

    card.innerHTML = `
      <div class="card-top">
        <div class="card-brand">
          <span class="brand-icon">${icon("model")}</span>
          <div class="brand-title">
            <h4>${escapeHtml(provider.display_name || provider.id)}</h4>
            <span class="brand-sub">${escapeHtml(provider.base_url || "默认 API Endpoint")}</span>
          </div>
        </div>
        ${isActive ? `<span class="badge-active">${icon("check")} 当前应用</span>` : ""}
      </div>
      <div class="card-mid">
        <div class="meta-row">
          <span>默认模型:</span> <strong>${escapeHtml(provider.default_model || "未设置")}</strong>
        </div>
        <div class="meta-row">
          <span>协议接口:</span> <span class="protocol-tag">${escapeHtml((provider.protocol || "auto").toUpperCase())}</span>
        </div>
        <div class="models-preview-wrap">
          ${modelsPreview || '<span class="no-models">空模型清单</span>'}
          ${moreCount}
        </div>
      </div>
      <div class="card-bottom">
        <button type="button" class="action-btn edit-provider-btn">
          ${icon("edit")} 配置参数 & 模型管理
        </button>
        ${!isActive ? `<button type="button" class="action-btn make-active-btn">${icon("check")} 设为活跃</button>` : ""}
      </div>
    `;

    // 4. 绑定编辑与激活交互
    card.querySelector(".edit-provider-btn").addEventListener("click", () => {
      openProviderEditor(provider, index, () => {
        renderProviders();
      });
    });

    const makeActiveBtn = card.querySelector(".make-active-btn");
    if (makeActiveBtn) {
      makeActiveBtn.addEventListener("click", () => {
        config.active_provider = provider.id;
        renderProviders();
      });
    }

    gridEl.appendChild(card);
  });

  paneEl.appendChild(gridEl);
}
