// ============================================================
// 命令类工具渲染器：run_command/background_command
// 摘要显示命令行，输出区以终端风格渲染
// ============================================================

import {
  toolIcons,
  parseArgs,
  renderTextOutput,
  renderErrorOutput,
  clipSummary,
} from "./common.js";

/**
 * 提取命令文本，兼容 command/cmd 等字段
 * @param {Object} args - 参数对象
 * @returns {string}
 */
function pickCommand(args) {
  return args.command || args.cmd || args.script || "";
}

const run = {
  icon: toolIcons.terminal,
  /**
   * 摘要：显示命令行
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const cmd = pickCommand(args);
    return cmd ? `$ ${clipSummary(cmd, 100)}` : "执行命令";
  },
  /**
   * 渲染输出：终端风格纯文本
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 8000);
  },
};

const background = {
  icon: toolIcons.terminal,
  /**
   * 摘要：显示后台命令
   * @param {Object} args - 参数
   * @returns {string}
   */
  summarize(args) {
    const cmd = pickCommand(args);
    return cmd ? `后台 $ ${clipSummary(cmd, 90)}` : "后台命令";
  },
  /**
   * 渲染输出：后台命令信息
   * @param {Object} ctx - 渲染上下文
   * @returns {void}
   */
  renderBody(ctx) {
    if (!ctx.ok) return renderErrorOutput(ctx.bodyEl, ctx.output);
    renderTextOutput(ctx.bodyEl, ctx.output, 4000);
  },
};

export default { run, background };
