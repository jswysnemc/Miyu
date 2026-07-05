// ============================================================
// 工具渲染器注册表：按工具名分发到具体渲染器
// 渲染器对象结构：{ icon, summarize(args, ctx), renderBody(ctx) }
// ctx 字段：{ name, args, output, ok, bodyEl, summaryEl }
// ============================================================

import { toolIcons, parseArgs } from "./common.js";
import fileTools from "./file-tools.js";
import command from "./command.js";
import web from "./web.js";
import image from "./image.js";
import memory from "./memory.js";
import kb from "./kb.js";
import meme from "./meme.js";
import divination from "./divination.js";
import system from "./system.js";
import heavy from "./heavy.js";
import load from "./load.js";
import defaultRenderer from "./default.js";

/**
 * 工具名到渲染器的映射表
 * @type {Record<string, Object>}
 */
const registry = {
  // 文件与代码
  read: fileTools.read,
  write: fileTools.write,
  edit: fileTools.edit,
  glob: fileTools.glob,
  grep: fileTools.grep,
  trash: fileTools.trash,

  // 命令
  run_command: command.run,
  background_command: command.background,
  shell_intercept: command.run,

  // 网络
  web_search: web.search,
  search_web: web.search,
  web_fetch: web.fetch,
  search_web_images: web.images,

  // 图像
  analyze_image: image.analyze,
  print_image: image.print,
  generate_image: image.generate,
  screenshot: image.print,

  // 记忆
  remember_fact: memory.remember,
  recall_memory: memory.recall,
  search_evicted: memory.evicted,

  // 知识库
  upload_kb: kb.upload,
  read_kb: kb.read,
  search_kb: kb.search,

  // 表情包
  search_meme: meme.search,
  show_meme: meme.show,
  add_meme: meme.add,

  // 玄学
  tarot: divination.tarot,
  zhouyi: divination.zhouyi,
  fortune_lot: divination.fortune,
  dice: divination.dice,

  // 系统
  alarm: system.alarm,
  weather: system.weather,
  exchange_rate: system.exchange,
  man: system.man,
  archlinux: system.arch,

  // 重型与子代理
  deep_research: heavy.research,
  deep_diagnose: heavy.diagnose,
  linux_game: heavy.game,
  task: heavy.task,

  // 工具加载
  load: load,
};

/**
 * 根据工具名获取渲染器，未注册时返回默认渲染器
 * @param {string} name - 工具名
 * @returns {Object} 渲染器对象
 */
export function getToolRenderer(name) {
  return registry[name] || defaultRenderer;
}

/**
 * 计算工具摘要文本
 * @param {string} name - 工具名
 * @param {string} argsRaw - 参数 JSON 字符串
 * @returns {string} 摘要文本
 */
export function summarizeTool(name, argsRaw) {
  const renderer = getToolRenderer(name);
  const args = parseArgs(argsRaw);
  try {
    return renderer.summarize(args, { name }) || "";
  } catch {
    return "";
  }
}

/**
 * 获取工具图标 SVG
 * @param {string} name - 工具名
 * @returns {string} SVG 字符串
 */
export function getToolIcon(name) {
  const renderer = getToolRenderer(name);
  return renderer.icon || toolIcons.package;
}

/**
 * 渲染工具输出区
 * @param {string} name - 工具名
 * @param {Object} ctx - 渲染上下文 { args, output, ok, bodyEl, summaryEl }
 * @returns {void}
 */
export function renderToolBody(name, ctx) {
  const renderer = getToolRenderer(name);
  try {
    renderer.renderBody(ctx);
  } catch (err) {
    // 1. 渲染器异常时降级为纯文本
    const pre = document.createElement("pre");
    pre.className = "tool-output-text";
    pre.textContent = String(ctx.output || "");
    ctx.bodyEl.innerHTML = "";
    ctx.bodyEl.appendChild(pre);
  }
}

export { toolIcons };
