// ============================================================
// 设置：系统工具运行配置面板
// 使用自研开关组件与沉浸字段卡片排版
// ============================================================

import { appState } from "../state.js";
import { field, checkbox, setupFormInteractions } from "./form-utils.js";
import { icon } from "../icons.js";

const paneEl = document.getElementById("settingsTools");

/**
 * 渲染工具运行控制台面板
 * @returns {void}
 */
export function renderTools() {
  if (!paneEl || !appState.config) return;
  const tools = appState.config.tools;

  paneEl.innerHTML = `
    <div class="settings-top-header">
      <span class="header-icon">${icon("tool")}</span>
      <div>
        <h3>系统工具集与本地环境执行参数 (Tools Runtime)</h3>
        <p>控制智能体调用本地文件修改、终端命令行执行及后台长期监控命令的权限与安全限额。</p>
      </div>
    </div>
    <div class="settings-cards-wrap">
      <div class="config-card">
        <div class="card-title">
          <h4>开关与流式控制</h4>
        </div>
        <div class="form-grid">
          ${checkbox("tools.enabled", "允许智能体主动调起系统工具", tools.enabled, "赋予智能体执行本地脚本与读写工作区文件的能力")}
          ${checkbox("tools.progressive_loading_enabled", "启用渐进式输出流式卡片渲染", tools.progressive_loading_enabled, "在工具执行过程中实时渲染终端回显与动态状态")}
          ${checkbox("tools.background_commands_enabled", "允许在背景运行长耗时终端进程", tools.background_commands_enabled, "将耗时编译与监控脚本作为异步任务派发至后台运行")}
        </div>
      </div>
      <div class="config-card">
        <div class="card-title">
          <h4>安全限制与超时阀值</h4>
        </div>
        <div class="form-grid">
          ${field("tools.max_rounds", "连续自动工具执行最大轮数", tools.max_rounds, "number", "", "输入单次会话连续执行轮数 (0 为不限)")}
          ${field("tools.background_command_timeout_seconds", "后台任务超时自动中止秒数", tools.background_command_timeout_seconds, "number", "", "输入后台进程超时秒数 (0 为不限)")}
          ${field("tools.background_command_log_max_bytes", "终端日志缓冲最大字节上限", tools.background_command_log_max_bytes, "number", "", "输入日志内存缓冲最大阈值 (字节)")}
        </div>
      </div>
    </div>
  `;

  // 激活自研开关组件交互
  setupFormInteractions(paneEl);
}
