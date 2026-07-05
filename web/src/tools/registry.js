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
  // 文件与目录
  read_file: fileTools.read,
  write_file: fileTools.write,
  edit_file: fileTools.edit,
  list_directory: fileTools.glob,
  create_directory: fileTools.write,
  get_current_directory: fileTools.glob,
  get_current_time: system.man,
  find_files: fileTools.glob,
  glob: fileTools.glob,
  search_text: fileTools.grep,
  grep: fileTools.grep,
  trash_path: fileTools.trash,

  // 命令
  run_command: command.run,
  background_command: command.background,
  shell_intercept: command.run,

  // 网络
  web_search: web.search,
  web_fetch: web.fetch,
  search_web_images: web.images,
  fcitx5_input_method_wiki_qurey: web.fetch,
  moegirl_query: web.fetch,

  // 图像
  analyze_image: image.analyze,
  vision_analyze: image.analyze,
  print_image: image.print,
  generate_image: image.generate,
  send_channel_image: image.print,
  send_channel_file: image.print,
  send_channel_video: image.print,

  // 记忆
  remember_fact: memory.remember,
  recall_memory: memory.recall,
  recall_memories: memory.recall,
  recall_past_events: memory.recall,
  search_evicted_context: memory.evicted,
  forget_memory: memory.remember,
  forget_memories: memory.remember,
  list_memory: memory.recall,
  list_memories: memory.recall,

  // 知识库
  upload_knowledge_base_file: kb.upload,
  upload_text_to_knowledge_base: kb.upload,
  read_knowledge_base_file: kb.read,
  search_knowledge_base: kb.search,
  search_knowledge_base_by_name: kb.search,
  edit_knowledge_base_file: kb.read,
  remove_knowledge_base_file: fileTools.trash,
  list_knowledge_base_files: kb.search,

  // 表情包
  search_meme: meme.search,
  show_meme: meme.show,
  add_meme: meme.add,
  update_meme: meme.add,
  delete_meme: meme.search,

  // 玄学
  draw_tarot_card: divination.tarot,
  draw_zhouyi_hexagram: divination.zhouyi,
  xuanxue_divine: divination.zhouyi,
  xuanxue_pick: divination.dice,
  draw_fortune_lot: divination.fortune,
  roll_dice: divination.dice,

  // 系统
  set_alarm: system.alarm,
  list_alarms: system.alarm,
  cancel_alarm: system.alarm,
  weather: system.weather,
  get_weather: system.weather,
  exchange_rate: system.exchange,
  get_exchange_rate: system.exchange,
  online_man_search: system.man,
  online_man_get_page: system.man,
  man_search: system.man,
  man_read: system.man,
  archlinux_official_package_query: system.arch,
  archwiki_query: system.arch,
  pacman_search: system.arch,
  aur_search_packages: system.arch,
  aur_get_package_info: system.arch,
  aur_check_status: system.arch,
  query_deepseek_status: system.arch,
  check_issue: system.man,
  check_os_info: system.man,
  protondb_query: system.arch,
  calculate: system.man,
  calculator: system.man,
  calculate_hash: system.man,
  decode_encoded_text: system.man,

  // 重型与子代理
  deep_research: heavy.research,
  deep_diagnose: heavy.research,
  linux_input_method_diagnose: heavy.research,
  linux_game_compatibility: heavy.game,
  gather_linux_game_compatibility_signals: heavy.game,
  register_linux_game_evidence: heavy.game,
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
