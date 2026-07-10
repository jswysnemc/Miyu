import type {
  ConfigResponse,
  FileContent,
  FileNode,
  GatewayStatus,
  GitDiff,
  HistoryEntry,
  RunInfo,
  Session,
  TerminalInfo,
  Workspace,
  WorkspaceList
} from "./contracts";

/** 使用 URL 启动令牌建立同源会话。 */
export async function bootstrapSession(): Promise<void> {
  const url = new URL(window.location.href);
  const token = url.searchParams.get("token");
  if (!token) return;
  const response = await fetch(`/api/auth/session?token=${encodeURIComponent(token)}`, {
    method: "POST",
    credentials: "same-origin"
  });
  if (!response.ok) throw new Error("Miyu Web 访问令牌无效");
  url.searchParams.delete("token");
  window.history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
}

/** 发送 JSON API 请求并统一处理错误。 */
export async function apiRequest<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    credentials: "same-origin",
    ...init,
    headers: {
      ...(init?.body ? { "Content-Type": "application/json" } : {}),
      ...init?.headers
    }
  });
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { error?: string } | null;
    throw new Error(body?.error ?? `HTTP ${response.status}`);
  }
  return response.json() as Promise<T>;
}

export const api = {
  workspaces: {
    list: () => apiRequest<WorkspaceList>("/api/workspaces"),
    add: (path: string, name?: string) =>
      apiRequest<Workspace>("/api/workspaces", {
        method: "POST",
        body: JSON.stringify({ path, name })
      }),
    switch: (id: string) => apiRequest<Workspace>(`/api/workspaces/${id}/switch`, { method: "POST" }),
    rename: (id: string, name: string) =>
      apiRequest<Workspace>(`/api/workspaces/${id}`, {
        method: "PATCH",
        body: JSON.stringify({ name })
      }),
    remove: (id: string) => apiRequest<{ removed: boolean }>(`/api/workspaces/${id}`, { method: "DELETE" })
  },
  sessions: {
    list: () => apiRequest<Session[]>("/api/sessions"),
    create: (title?: string) =>
      apiRequest<Session>("/api/sessions", { method: "POST", body: JSON.stringify({ title }) }),
    switch: (id: string) => apiRequest<Session>(`/api/sessions/${id}/switch`, { method: "POST" }),
    rename: (id: string, title: string) =>
      apiRequest<Session>(`/api/sessions/${id}`, { method: "PATCH", body: JSON.stringify({ title }) }),
    remove: (id: string) => apiRequest<{ deleted: boolean }>(`/api/sessions/${id}`, { method: "DELETE" }),
    messages: (id: string) => apiRequest<HistoryEntry[]>(`/api/sessions/${id}/messages?limit=500`)
  },
  runs: {
    start: (sessionId: string, input: string, mode: "plan" | "yolo") =>
      apiRequest<RunInfo>("/api/runs", {
        method: "POST",
        body: JSON.stringify({ session_id: sessionId, input, mode })
      }),
    stop: (id: string) => apiRequest<{ stopped: boolean }>(`/api/runs/${id}`, { method: "DELETE" })
  },
  workspace: {
    tree: () => apiRequest<FileNode[]>("/api/workspace/tree?depth=5"),
    file: (path: string) => apiRequest<FileContent>(`/api/workspace/file?path=${encodeURIComponent(path)}`),
    save: (path: string, content: string) =>
      apiRequest<FileContent>("/api/workspace/file", {
        method: "PUT",
        body: JSON.stringify({ path, content })
      }),
    diff: () => apiRequest<GitDiff>("/api/workspace/diff")
  },
  config: {
    load: () => apiRequest<ConfigResponse>("/api/config"),
    save: (config: Record<string, unknown>) =>
      apiRequest<ConfigResponse>("/api/config", { method: "PUT", body: JSON.stringify(config) })
  },
  gateways: {
    list: () => apiRequest<GatewayStatus[]>("/api/gateways"),
    start: (id: string) => apiRequest<Record<string, unknown>>(`/api/gateways/${id}/start`, { method: "POST" }),
    stop: (id: string) => apiRequest<Record<string, unknown>>(`/api/gateways/${id}/stop`, { method: "POST" })
  },
  terminals: {
    list: () => apiRequest<{ terminals: TerminalInfo[] }>("/api/terminals"),
    create: (cols: number, rows: number) =>
      apiRequest<TerminalInfo>("/api/terminals", { method: "POST", body: JSON.stringify({ cols, rows }) }),
    remove: (id: string) => apiRequest<{ removed: boolean }>(`/api/terminals/${id}`, { method: "DELETE" })
  }
};
