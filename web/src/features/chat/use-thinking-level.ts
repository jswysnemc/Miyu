import { useEffect, useState } from "react";
import type { ThinkingLevel } from "../../api/contracts";

const THINKING_LEVEL_STORAGE_KEY = "miyu.thinking-level";
const THINKING_LEVELS: ThinkingLevel[] = ["auto", "none", "low", "medium", "high", "xhigh", "max"];

/**
 * 管理思考等级和浏览器本地偏好。
 *
 * @returns 当前思考等级和更新方法
 */
export function useThinkingLevel() {
  const [thinkingLevel, setThinkingLevel] = useState<ThinkingLevel>(loadThinkingLevel);

  useEffect(() => {
    window.localStorage.setItem(THINKING_LEVEL_STORAGE_KEY, thinkingLevel);
  }, [thinkingLevel]);

  return { thinkingLevel, setThinkingLevel };
}

/**
 * 读取本地保存的思考等级。
 *
 * @returns 合法的思考等级，缺失时返回自动
 */
function loadThinkingLevel(): ThinkingLevel {
  const stored = window.localStorage.getItem(THINKING_LEVEL_STORAGE_KEY) as ThinkingLevel | null;
  return stored && THINKING_LEVELS.includes(stored) ? stored : "auto";
}
