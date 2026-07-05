// ============================================================
// 流式增量渲染：基于 streaming-markdown (smd) 处理 chunk
// smd 不可用时降级为纯文本累积；finalize 时用 marked 重渲染保证准确
// 思考过程按需 attach，避免纯正文消息出现空思考区
// ============================================================

import { unwrapMathFromCode } from "./math.js";
import { getCodeFenceCount, renderCodeBlockShells } from "./code.js";
import { renderCompletedBlocks, renderFull } from "./renderer.js";

/**
 * 创建流式渲染器
 * @param {HTMLElement} contentEl - 正文内容容器
 * @returns {{writeContent: Function, attachThought: Function, writeThought: Function, finalize: Function}}
 */
export function createStreamRenderer(contentEl) {
  const smd = window.smd || null;
  let contentParser = null;
  let thoughtParser = null;
  let thoughtEl = null;
  let fullContent = "";
  let fullThought = "";
  let contentFenceCount = 0;
  let thoughtFenceCount = 0;
  let lastEnhanceTime = 0;

  /**
   * 懒创建正文 parser
   * @returns {void}
   */
  function ensureContentParser() {
    if (contentParser) return;
    if (smd) {
      contentParser = smd.parser(smd.default_renderer(contentEl));
    } else {
      contentEl.style.whiteSpace = "pre-wrap";
    }
  }

  /**
   * 节流增强：每 600ms 对已闭合块补充渲染
   * @param {HTMLElement} el - 容器
   * @returns {void}
   */
  function enhanceThrottled(el) {
    const now = Date.now();
    if (now - lastEnhanceTime > 600) {
      lastEnhanceTime = now;
      renderCompletedBlocks(el, true);
    }
  }

  /**
   * 写入正文增量
   * @param {string} chunk - 增量文本
   * @returns {void}
   */
  function writeContent(chunk) {
    if (!chunk) return;
    ensureContentParser();
    fullContent += chunk;
    contentEl.setAttribute("data-raw", fullContent);
    if (contentParser) {
      // 1. smd 增量写入，剥离公式反引号
      smd.parser_write(contentParser, unwrapMathFromCode(chunk));
      // 2. 立即渲染代码块外壳
      renderCodeBlockShells(contentEl, true);
      // 3. 代码块闭合时触发增强渲染
      const fence = getCodeFenceCount(fullContent);
      if (fence > contentFenceCount && fence % 2 === 0) {
        contentFenceCount = fence;
        renderCompletedBlocks(contentEl, true);
      }
    } else {
      contentEl.textContent = fullContent;
    }
    enhanceThrottled(contentEl);
  }

  /**
   * 挂载思考过程容器并创建对应 parser
   * @param {HTMLElement} el - 思考内容容器
   * @returns {void}
   */
  function attachThought(el) {
    if (thoughtParser || !el) return;
    thoughtEl = el;
    if (smd) {
      thoughtParser = smd.parser(smd.default_renderer(thoughtEl));
    } else {
      thoughtEl.style.whiteSpace = "pre-wrap";
    }
  }

  /**
   * 写入思考过程增量
   * @param {string} chunk - 增量文本
   * @returns {void}
   */
  function writeThought(chunk) {
    if (!chunk || !thoughtEl) return;
    fullThought += chunk;
    thoughtEl.setAttribute("data-raw", fullThought);
    if (thoughtParser) {
      smd.parser_write(thoughtParser, unwrapMathFromCode(chunk));
      renderCodeBlockShells(thoughtEl, true);
      const fence = getCodeFenceCount(fullThought);
      if (fence > thoughtFenceCount && fence % 2 === 0) {
        thoughtFenceCount = fence;
        renderCompletedBlocks(thoughtEl, true);
      }
    } else {
      thoughtEl.textContent = fullThought;
    }
    enhanceThrottled(thoughtEl);
  }

  /**
   * 结束流式，用完整文本 marked 重渲染保证准确
   * @param {string|null} finalContent - done 事件的完整正文
   * @param {string|null} finalThought - done 事件的完整思考
   * @returns {void}
   */
  function finalize(finalContent, finalThought) {
    // 1. 结束 smd 解析器
    if (contentParser) {
      try {
        smd.parser_end(contentParser);
      } catch (err) {
        console.info("【前端/流式】smd parser_end 异常", err);
      }
    }
    if (thoughtParser) {
      try {
        smd.parser_end(thoughtParser);
      } catch (err) {
        console.info("【前端/流式】smd thought parser_end 异常", err);
      }
    }
    // 2. 用完整文本 marked 重渲染
    const text = finalContent != null ? finalContent : fullContent;
    if (text) renderFull(contentEl, text);
    if (thoughtEl && (finalThought || fullThought)) {
      renderFull(thoughtEl, finalThought || fullThought);
    }
  }

  return { writeContent, attachThought, writeThought, finalize };
}
