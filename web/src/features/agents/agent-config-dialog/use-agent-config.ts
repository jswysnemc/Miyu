import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "../../../api/client";
import type { AppConfig } from "../../../api/contracts";

export type AgentConfigDraft = {
  defaultAgent: string;
  subagentProviderId: string;
  subagentModel: string;
};

/**
 * 从应用配置读取 agent 相关字段草稿。
 *
 * @param config 应用配置
 * @returns agent 配置草稿
 */
export function readAgentConfigDraft(config: AppConfig): AgentConfigDraft {
  return {
    defaultAgent: config.default_agent ?? "default",
    subagentProviderId: config.subagent?.provider_id ?? "",
    subagentModel: config.subagent?.model ?? ""
  };
}

/**
 * 管理 agent 配置弹窗的读取与保存。
 *
 * 复用 config 查询键,保存后失效缓存,让主界面 agent 选择器同步刷新。
 *
 * @returns 配置、加载状态与保存方法
 */
export function useAgentConfig() {
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: ["config"], queryFn: api.config.load });
  const save = useMutation({
    mutationFn: (draft: AgentConfigDraft) => {
      const current = query.data?.config;
      if (!current) throw new Error("配置尚未加载");
      const next: AppConfig = {
        ...current,
        default_agent: draft.defaultAgent === "default" ? null : draft.defaultAgent,
        subagent: { provider_id: draft.subagentProviderId, model: draft.subagentModel }
      };
      return api.config.save(next as unknown as Record<string, unknown>);
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["config"] })
  });
  return {
    config: query.data?.config ?? null,
    isLoading: query.isLoading,
    error: query.error,
    save: save.mutateAsync,
    saving: save.isPending,
    saveError: save.error
  };
}
