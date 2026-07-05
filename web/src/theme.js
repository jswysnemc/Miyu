// ============================================================
// 主题切换：明暗双主题 + 代码高亮主题联动 + 持久化
// ============================================================

import { icons } from "./icons.js";

/**
 * 读取当前主题
 * @returns {"light"|"dark"} 当前主题名
 */
export function currentTheme() {
  return document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light";
}

/**
 * 应用主题到文档根节点，并联动切换 highlight.js 主题样式表
 * @param {"light"|"dark"} theme - 目标主题
 * @returns {void}
 */
function applyTheme(theme) {
  const isDark = theme === "dark";
  document.documentElement.setAttribute("data-theme", isDark ? "dark" : "light");
  localStorage.setItem("miyu.web.theme", theme);
  // 1. 联动 highlight.js 明暗主题样式表
  const lightLink = document.getElementById("hljsLightTheme");
  const darkLink = document.getElementById("hljsDarkTheme");
  if (lightLink && darkLink) {
    lightLink.disabled = isDark;
    darkLink.disabled = !isDark;
  }
  // 2. 同步切换按钮图标
  const btn = document.getElementById("themeToggleBtn");
  if (btn) {
    btn.innerHTML = isDark ? icons.sun : icons.moon;
    btn.title = isDark ? "切换浅色模式" : "切换深色模式";
  }
  // 3. 重新初始化 Mermaid 主题并触发聊天区重绘
  if (window.mermaid && typeof window.mermaid.initialize === "function") {
    try {
      window.mermaid.initialize(mermaidConfig());
    } catch (err) {
      console.info("【前端/主题】Mermaid 主题初始化失败", err);
    }
  }
  window.dispatchEvent(new CustomEvent("miyu:theme-change", { detail: { theme } }));
}

/**
 * 切换明暗主题
 * @returns {void}
 */
export function toggleTheme() {
  applyTheme(currentTheme() === "dark" ? "light" : "dark");
}

/**
 * 初始化主题：读取本地存储，默认浅色
 * @returns {void}
 */
export function initTheme() {
  const saved = localStorage.getItem("miyu.web.theme") || "light";
  applyTheme(saved);
}

/**
 * 提供 Mermaid 当前主题配置
 * @returns {Object} Mermaid 初始化配置
 */
export function mermaidConfig() {
  const isDark = currentTheme() === "dark";
  return {
    startOnLoad: false,
    theme: "base",
    securityLevel: "loose",
    suppressErrorRendering: true,
    themeVariables: isDark
      ? {
          background: "transparent",
          primaryColor: "#1e293b",
          primaryTextColor: "#e0e7ff",
          primaryBorderColor: "#818cf8",
          lineColor: "#94a3b8",
          textColor: "#e2e8f0",
        }
      : {
          background: "transparent",
          primaryColor: "#e5f1e9",
          primaryTextColor: "#174832",
          primaryBorderColor: "#215d43",
          lineColor: "#6f746f",
          textColor: "#171b1f",
        },
  };
}
