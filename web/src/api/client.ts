import type {
  ConfigResponse,
  DirectoryListing,
  FileContent,
  FileMutation,
  FileNode,
  GatewayStatus,
  GitDiff,
  HistoryEntry,
  PromptDocument,
  PromptKind,
  PromptSummary,
  ProviderConfig,
  ProviderModelsResponse,
  RunModelSelection,
  ThinkingLevel,
  RunInfo,
  SessionTimelineTurn,
  SystemUsage,
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
    browse: (path?: string) => apiRequest<DirectoryListing>(`/api/workspaces/browse${path ? `?path=${encodeURIComponent(path)}` : ""}`),
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
    removeMany: (ids: string[]) =>
      apiRequest<{ deleted_ids: string[] }>("/api/sessions/bulk-delete", {
        method: "POST",
        body: JSON.stringify({ ids })
      }),
    messages: (id: string) => apiRequest<HistoryEntry[]>(`/api/sessions/${id}/messages?limit=500`),
    timeline: (id: string) => apiRequest<SessionTimelineTurn[]>(`/api/sessions/${id}/timeline?limit=500`)
  },
  runs: {
    start: (
      sessionId: string,
      input: string,
      mode: "plan" | "yolo",
      selection?: RunModelSelection,
      imageUrls?: string[],
      thinkingLevel?: ThinkingLevel
    ) =>
      apiRequest<RunInfo>("/api/runs", {
        method: "POST",
        body: JSON.stringify({
          session_id: sessionId,
          input,
          mode,
          provider_id: selection?.providerId,
          model: selection?.model,
          image_urls: imageUrls,
          thinking_level: thinkingLevel
        })
      }),
    stop: (id: string) => apiRequest<{ stopped: boolean }>(`/api/runs/${id}`, { method: "DELETE" })
  },
  workspace: {
    tree: () => apiRequest<FileNode[]>("/api/workspace/tree?depth=5"),
    file: (path: string) => apiRequest<FileContent>(`/api/workspace/file?path=${encodeURIComponent(path)}`),
    save: (path: string, content: string, expectedModifiedAt?: number | null) =>
      apiRequest<FileContent>("/api/workspace/file", {
        method: "PUT",
        body: JSON.stringify({ path, content, expected_modified_at: expectedModifiedAt })
      }),
    create: (path: string, kind: "file" | "directory") =>
      apiRequest<FileMutation>("/api/workspace/entry", {
        method: "POST",
        body: JSON.stringify({ path, kind })
      }),
    rename: (from: string, to: string) =>
      apiRequest<FileMutation>("/api/workspace/entry", {
        method: "PATCH",
        body: JSON.stringify({ from, to })
      }),
    remove: (path: string) =>
      apiRequest<FileMutation>("/api/workspace/entry", {
        method: "DELETE",
        body: JSON.stringify({ path })
      }),
    diff: () => apiRequest<GitDiff>("/api/workspace/diff"),
    gitAction: (action: "init" | "stage" | "unstage" | "discard" | "commit", paths: string[] = [], message?: string) =>
      apiRequest<GitDiff>("/api/workspace/git", {
        method: "POST",
        body: JSON.stringify({ action, paths, message })
      })
  },
  config: {
    load: () => apiRequest<ConfigResponse>("/api/config"),
    save: (config: Record<string, unknown>) =>
      apiRequest<ConfigResponse>("/api/config", { method: "PUT", body: JSON.stringify(config) })
  },
  providers: {
    models: (provider: ProviderConfig) =>
      apiRequest<ProviderModelsResponse>("/api/providers/models", {
        method: "POST",
        body: JSON.stringify({ provider })
      })
  },
  prompts: {
    list: (kind: PromptKind) => apiRequest<{ items: PromptSummary[] }>(`/api/prompts/${kind}`),
    read: (kind: PromptKind, name: string) => apiRequest<PromptDocument>(`/api/prompts/${kind}/${encodeURIComponent(name)}`),
    create: (kind: PromptKind, name: string, content: string) =>
      apiRequest<PromptDocument>(`/api/prompts/${kind}`, {
        method: "POST",
        body: JSON.stringify({ name, content })
      }),
    update: (kind: PromptKind, currentName: string, name: string, content: string) =>
      apiRequest<PromptDocument>(`/api/prompts/${kind}/${encodeURIComponent(currentName)}`, {
        method: "PUT",
        body: JSON.stringify({ name, content })
      }),
    remove: (kind: PromptKind, name: string) =>
      apiRequest<{ removed: boolean }>(`/api/prompts/${kind}/${encodeURIComponent(name)}`, { method: "DELETE" })
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
  },
  system: {
    usage: () => apiRequest<SystemUsage>("/api/system/usage")
  }
};
