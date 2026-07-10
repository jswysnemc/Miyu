export type Workspace = {
  id: string;
  name: string;
  path: string;
  last_opened_at: string;
};

export type WorkspaceList = {
  active_id: string;
  workspaces: Workspace[];
};

export type Session = {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  active: boolean;
};

export type HistoryEntry = {
  timestamp: string;
  role: string;
  content: string;
  reasoning?: string | null;
};

export type FileNode = {
  name: string;
  path: string;
  kind: "file" | "directory" | "symlink";
  children: FileNode[];
};

export type FileContent = {
  path: string;
  content: string;
  size: number;
  modified_at?: number | null;
};

export type GitDiff = {
  repository: boolean;
  status: string;
  diff: string;
};

export type GatewayStatus = {
  id: string;
  title: string;
  enabled: boolean;
  task_id?: string | null;
  status: string;
  pid?: number | null;
};

export type ConfigResponse = {
  config: Record<string, unknown>;
  secret_sentinel: string;
};

export type RunInfo = {
  run_id: string;
  workspace_id: string;
  session_id: string;
};

export type WebEvent = {
  sequence: number;
  run_id: string;
  workspace_id: string;
  session_id: string;
  timestamp: string;
  type: string;
  payload: Record<string, unknown>;
};

export type TerminalInfo = {
  id: string;
  title: string;
  cols: number;
  rows: number;
};
