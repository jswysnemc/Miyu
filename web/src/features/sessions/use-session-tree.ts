import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { api } from "../../api/client";

const EXPANDED_STORAGE_KEY = "miyu.session-tree-expanded";

/**
 * 管理工作区会话树、展开状态和运行中会话集合。
 *
 * @returns 会话树查询、展开操作和运行状态
 */
export function useSessionTree() {
  const tree = useQuery({ queryKey: ["session-tree"], queryFn: api.sessions.tree });
  const runs = useQuery({
    queryKey: ["active-runs"],
    queryFn: api.runs.active,
    refetchInterval: 1500
  });
  const [expanded, setExpanded] = useState<Set<string>>(() => {
    try {
      return new Set(JSON.parse(window.localStorage.getItem(EXPANDED_STORAGE_KEY) ?? "[]") as string[]);
    } catch {
      return new Set();
    }
  });

  useEffect(() => {
    const activeId = tree.data?.find((workspace) => workspace.active)?.workspace_id;
    if (!activeId) return;
    setExpanded((current) => current.has(activeId) ? current : new Set([...current, activeId]));
  }, [tree.data]);

  useEffect(() => {
    window.localStorage.setItem(EXPANDED_STORAGE_KEY, JSON.stringify(Array.from(expanded)));
  }, [expanded]);

  const runningSessions = useMemo(
    () => new Set((runs.data?.runs ?? []).filter((run) => run.status === "running" || run.status === "queued").map((run) => `${run.workspace_id}:${run.session_id}`)),
    [runs.data]
  );

  /** 切换工作区节点的展开状态。 */
  const toggleWorkspace = (workspaceId: string) => {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(workspaceId)) next.delete(workspaceId);
      else next.add(workspaceId);
      return next;
    });
  };

  return { tree, expanded, runningSessions, toggleWorkspace };
}
