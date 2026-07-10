import type { ITerminalOptions, ITheme } from "@xterm/xterm";

export const TERMINAL_FONT_FAMILY = '"Fira Code", "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace';

/**
 * 创建终端显示和交互选项。
 *
 * @returns xterm 终端选项
 */
export function createTerminalOptions(): ITerminalOptions {
  return {
    fontFamily: TERMINAL_FONT_FAMILY,
    fontSize: 12,
    fontWeight: "400",
    fontWeightBold: "500",
    letterSpacing: 0,
    lineHeight: 1.3,
    cursorBlink: true,
    theme: createTerminalTheme()
  };
}

/**
 * 从 CSS 主题令牌读取终端配色，保证终端背景与代码块、工具视图一致。
 *
 * @returns xterm 颜色主题
 */
export function createTerminalTheme(): ITheme {
  const tokens = getComputedStyle(document.documentElement);
  // 1. 读取统一技术表面令牌，缺失时回退到内置深色值
  const read = (name: string, fallback: string) => tokens.getPropertyValue(name).trim() || fallback;
  return {
    background: read("--terminal-surface", "#101412"),
    foreground: read("--terminal-text", "#e5e9e6"),
    cursor: read("--signal", "#9cbfb5"),
    selectionBackground: read("--terminal-selection", "#2b3934")
  };
}
