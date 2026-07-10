import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import type { RunModelSelection } from "../../api/contracts";
import { api } from "../../api/client";
import { buildChatModelChoices, resolveChatModelSelection } from "./chat-model-options";

const CHAT_MODEL_STORAGE_KEY = "miyu.chat-model";

/**
 * 管理输入区模型列表、当前选择和本地偏好。
 *
 * @returns 模型查询状态、选项和选择方法
 */
export function useChatModel() {
  const response = useQuery({ queryKey: ["config"], queryFn: api.config.load });
  const [preferred, setPreferred] = useState<RunModelSelection | null>(loadStoredSelection);
  const choices = response.data ? buildChatModelChoices(response.data.config) : [];
  const selection = response.data ? resolveChatModelSelection(response.data.config, preferred) : null;

  useEffect(() => {
    if (!selection) return;
    window.localStorage.setItem(CHAT_MODEL_STORAGE_KEY, JSON.stringify(selection));
  }, [selection?.providerId, selection?.model]);

  /**
   * 更新本轮使用的供应商和模型。
   *
   * @param next 新模型选择
   */
  const selectModel = (next: RunModelSelection) => setPreferred(next);

  return { choices, selection, selectModel, isLoading: response.isLoading, error: response.error };
}

/**
 * 读取本地保存的模型偏好。
 *
 * @returns 已保存的供应商和模型，内容非法时返回空值
 */
function loadStoredSelection(): RunModelSelection | null {
  try {
    const value = JSON.parse(window.localStorage.getItem(CHAT_MODEL_STORAGE_KEY) ?? "null") as Partial<RunModelSelection> | null;
    if (value?.providerId && value.model) return { providerId: value.providerId, model: value.model };
  } catch {
    return null;
  }
  return null;
}
