// ============================================================
// 服务商精细编辑器：负责单个 Provider 参数修改
// 采用 Slide-over 抽屉形态，绑定 CustomSelect 与 TagEditor 组件
// ============================================================

import { escapeHtml, setByPath } from "./form-utils.js";
import { createCustomSelect } from "../components/custom-select.js";
import { createTagEditor } from "../components/tag-editor.js";
import { icon } from "../icons.js";

let editorDrawerEl = null;

/**
 * 确保抽屉容器存在
 * @returns {HTMLElement}
 */
function ensureDrawer() {
  if (!editorDrawerEl) {
    editorDrawerEl = document.createElement("div");
    editorDrawerEl.className = "provider-editor-drawer";
    editorDrawerEl.hidden = true;
    document.body.appendChild(editorDrawerEl);
  }
  return editorDrawerEl;
}

/**
 * 打开并渲染指定服务商编辑抽屉
 * @param {Object} provider - 服务商配置对象
 * @param {number} index - 在数组中的索引
 * @param {Function} onSave - 保存回调
 * @returns {void}
 */
export function openProviderEditor(provider, index, onSave) {
  const drawer = ensureDrawer();
  drawer.hidden = false;
  drawer.innerHTML = "";

  const overlay = document.createElement("div");
  overlay.className = "editor-drawer-overlay";
  overlay.addEventListener("click", () => closeProviderEditor());

  const panel = document.createElement("div");
  panel.className = "editor-drawer-panel";

  // 1. 标题栏
  panel.innerHTML = `
    <div class="drawer-header">
      <div class="header-title">
        <span class="provider-icon">${icon("model")}</span>
        <div>
          <h3>编辑提供商: ${escapeHtml(provider.display_name || provider.id)}</h3>
          <span>ID: ${escapeHtml(provider.id)}</span>
        </div>
      </div>
      <button class="icon-button" id="closeDrawerBtn" title="关闭">${icon("close")}</button>
    </div>
    <div class="drawer-body" id="drawerFormBody"></div>
    <div class="drawer-footer">
      <button class="ghost-button" id="cancelDrawerBtn">取消</button>
      <button class="send-button" id="confirmDrawerBtn">${icon("check")} 完成更改</button>
    </div>
  `;

  drawer.appendChild(overlay);
  drawer.appendChild(panel);

  const bodyEl = panel.querySelector("#drawerFormBody");

  // 2. 挂载基础字段
  bodyEl.innerHTML = `
    <div class="form-section">
      <h4>连接设置</h4>
      <div class="form-grid">
        <div class="custom-field-wrap">
          <div class="field-label-row"><span class="field-title">名称 (Display Name)</span></div>
          <div class="custom-input-container"><input class="custom-input-field" type="text" id="editDisplayName" value="${escapeHtml(provider.display_name || "")}"></div>
        </div>
        <div class="custom-field-wrap">
          <div class="field-label-row"><span class="field-title">Base URL</span></div>
          <div class="custom-input-container"><input class="custom-input-field" type="text" id="editBaseUrl" value="${escapeHtml(provider.base_url || "")}"></div>
        </div>
        <div class="custom-field-wrap">
          <div class="field-label-row"><span class="field-title">API Key</span></div>
          <div class="custom-input-container"><input class="custom-input-field" type="password" id="editApiKey" placeholder="留空则保持原密钥" value="${escapeHtml(provider.api_key || "")}"></div>
        </div>
      </div>
    </div>
    <div class="form-section">
      <h4>行为与推理配置</h4>
      <div class="form-grid" id="behaviorGrid">
        <div class="custom-field-wrap" id="fieldProtocol">
          <div class="field-label-row"><span class="field-title">传输协议 (Protocol)</span></div>
        </div>
        <div class="custom-field-wrap" id="fieldThinkingLevel">
          <div class="field-label-row"><span class="field-title">思考等级 (Thinking Level)</span></div>
        </div>
        <div class="custom-field-wrap" id="fieldThinkingFormat">
          <div class="field-label-row"><span class="field-title">思考解析格式 (Thinking Format)</span></div>
        </div>
        <div class="custom-field-wrap">
          <div class="field-label-row"><span class="field-title">超时时间 (秒)</span></div>
          <div class="custom-input-container"><input class="custom-input-field" type="number" id="editTimeout" value="${provider.timeout_seconds || 120}"></div>
        </div>
        <div class="custom-field-wrap">
          <div class="field-label-row"><span class="field-title">默认温度 (Temperature)</span></div>
          <div class="custom-input-container"><input class="custom-input-field" type="number" step="0.1" id="editTemperature" value="${provider.temperature ?? 0.7}"></div>
        </div>
        <div class="custom-field-wrap">
          <div class="field-label-row"><span class="field-title">默认选定模型</span></div>
          <div class="custom-input-container"><input class="custom-input-field" type="text" id="editDefaultModel" value="${escapeHtml(provider.default_model || "")}"></div>
        </div>
      </div>
    </div>
    <div class="form-section">
      <h4>支持模型阵列管理 (Tag Chips)</h4>
      <div class="custom-field-wrap full" id="fieldModels"></div>
    </div>
  `;

  // 3. 注入来自 custom-select.js 与 tag-editor.js 的无障碍自研 UI
  const protocolSelect = createCustomSelect({
    options: [
      { value: "auto", label: "自动判定 (Auto)" },
      { value: "openai", label: "OpenAI 兼容规范" },
      { value: "anthropic", label: "Anthropic Messages" },
      { value: "gemini", label: "Google Gemini Native" },
    ],
    value: provider.protocol || "auto",
  });
  bodyEl.querySelector("#fieldProtocol").appendChild(protocolSelect);

  const thinkingLevelSelect = createCustomSelect({
    options: [
      { value: "auto", label: "自动 (Auto)" },
      { value: "none", label: "关闭 (None)" },
      { value: "low", label: "低级推理 (Low)" },
      { value: "medium", label: "中度推理 (Medium)" },
      { value: "high", label: "深度推理 (High)" },
    ],
    value: provider.thinking_level || "auto",
  });
  bodyEl.querySelector("#fieldThinkingLevel").appendChild(thinkingLevelSelect);

  const thinkingFormatSelect = createCustomSelect({
    options: [
      { value: "auto", label: "自动格式 (Auto)" },
      { value: "deepseek", label: "DeepSeek R1 (<think> 标签)" },
      { value: "gemini", label: "Gemini Thought Block" },
      { value: "none", label: "无思维链" },
    ],
    value: provider.thinking_format || "auto",
  });
  bodyEl.querySelector("#fieldThinkingFormat").appendChild(thinkingFormatSelect);

  const tagEditor = createTagEditor({
    tags: provider.models || [],
    placeholder: "输入模型名如 gpt-4o 或 gemini-3 并按回车...",
  });
  bodyEl.querySelector("#fieldModels").appendChild(tagEditor);

  // 4. 事件绑定
  panel.querySelector("#closeDrawerBtn").addEventListener("click", () => closeProviderEditor());
  panel.querySelector("#cancelDrawerBtn").addEventListener("click", () => closeProviderEditor());
  
  panel.querySelector("#confirmDrawerBtn").addEventListener("click", () => {
    provider.display_name = panel.querySelector("#editDisplayName").value.trim();
    provider.base_url = panel.querySelector("#editBaseUrl").value.trim();
    const newKey = panel.querySelector("#editApiKey").value;
    if (newKey) provider.api_key = newKey;
    
    provider.protocol = protocolSelect.getValue();
    provider.thinking_level = thinkingLevelSelect.getValue();
    provider.thinking_format = thinkingFormatSelect.getValue();
    
    provider.timeout_seconds = Number.parseInt(panel.querySelector("#editTimeout").value, 10) || 120;
    provider.temperature = Number.parseFloat(panel.querySelector("#editTemperature").value) || 0.7;
    provider.default_model = panel.querySelector("#editDefaultModel").value.trim();
    provider.models = tagEditor.getTags();

    if (typeof onSave === "function") {
      onSave(provider, index);
    }
    closeProviderEditor();
  });

  requestAnimationFrame(() => {
    drawer.classList.add("show");
  });
}

/**
 * 关闭编辑抽屉
 * @returns {void}
 */
export function closeProviderEditor() {
  if (!editorDrawerEl) return;
  editorDrawerEl.classList.remove("show");
  setTimeout(() => {
    editorDrawerEl.hidden = true;
  }, 250);
}
