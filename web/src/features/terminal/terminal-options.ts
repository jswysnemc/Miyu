import type { ITerminalOptions, ITheme } from "@xterm/xterm";

export const TERMINAL_FONT_FAMILY = '"Fira Code", "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace';

/**
 * 创建终端显示和交互选项。
 *
 * @returns xterm 终端选项
 */
export function createTerminalOptions(): ITerminalOptions {
  const theme = document.documentElement.dataset.theme;
  const dark = theme === "graphite" || theme === "ocean" || (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  return {
    fontFamily: TERMINAL_FONT_FAMILY,
    fontSize: 12,
    fontWeight: "400",
    fontWeightBold: "500",
    letterSpacing: 0,
    lineHeight: 1.3,
    cursorBlink: true,
    theme: createTerminalTheme(dark)
  };
}

/**
 * 按颜色模式创建终端主题。
 *
 * @param dark 是否使用深色主题
 * @returns xterm 颜色主题
 */
export function createTerminalTheme(dark: boolean): ITheme {
  return dark
    ? { background: "#101412", foreground: "#e5e9e6", cursor: "#9cbfb5", selectionBackground: "#2b3934" }
    : { background: "#171c1e", foreground: "#e4e9e9", cursor: "#9dc2b8", selectionBackground: "#344247" };
}
