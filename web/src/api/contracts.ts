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

export type TimelineMessage = {
  timestamp: string;
  content: string;
  reasoning?: string | null;
};

export type TimelineToolEntry = {
  id: string;
  name: string;
  arguments: string;
  status: "running" | "completed" | "failed";
  output: string;
  ok?: boolean | null;
  error?: string | null;
  result_ref?: string | null;
  original_chars?: number | null;
  created_at: string;
  completed_at?: string | null;
};

export type SessionTimelineTurn = {
  turn_id: string;
  seq: number;
  status: "running" | "completed" | "interrupted";
  user: TimelineMessage;
  assistant: TimelineMessage;
  tools: TimelineToolEntry[];
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
  branch: string;
  status: string;
  files: GitFileStatus[];
  diff: string;
};

export type GitFileStatus = {
  path: string;
  index_status: string;
  worktree_status: string;
};

export type FileMutation = {
  path: string;
  kind: "file" | "directory";
};

export type PromptKind = "personas" | "identities";

export type PromptSummary = {
  name: string;
};

export type PromptDocument = PromptSummary & {
  content: string;
};

export type GatewayStatus = {
  id: string;
  title: string;
  enabled: boolean;
  task_id?: string | null;
  status: string;
  pid?: number | null;
};

export type ProviderConfig = {
  id: string;
  display_name: string;
  base_url: string;
  api_key?: string;
  protocol?: string;
  models?: string[];
  default_model?: string;
  thinking_level?: string;
  thinking_format?: string;
  timeout_seconds?: number;
  temperature?: number;
  anthropic_max_tokens?: number;
  extra_body?: string;
  model_context_chars?: Record<string, number>;
  model_metadata?: Record<string, ModelMetadata>;
  [key: string]: unknown;
};

export type ModelMetadata = {
  context_chars?: number;
  tools_enabled?: boolean;
  tags?: string[];
};

export type QqGatewayConfig = {
  enabled: boolean;
  transport: string;
  listen: string;
  base_url: string;
  token: string;
  app_id: string;
  client_secret: string;
  [key: string]: unknown;
};

export type WeixinGatewayConfig = {
  enabled: boolean;
  base_url: string;
  cdn_base_url: string;
  bot_type: string;
  token: string;
  account: string;
  bot_agent: string;
  [key: string]: unknown;
};

export type GatewayConfig = {
  qq: QqGatewayConfig;
  weixin: WeixinGatewayConfig;
  [key: string]: unknown;
};

export type AppConfig = {
  active_provider: string;
  providers: ProviderConfig[];
  gateways: GatewayConfig;
  plugins?: Record<string, Record<string, unknown>>;
  prompt?: {
    prompts_dir?: string;
    identities_dir?: string;
    user_identity_file?: string;
    active_persona?: string;
    active_identity?: string;
    [key: string]: unknown;
  };
  tools?: Record<string, unknown>;
  skills?: Record<string, unknown>;
  display?: Record<string, unknown>;
  context?: Record<string, unknown>;
  [key: string]: unknown;
};

export type ConfigResponse = {
  config: AppConfig;
  secret_sentinel: string;
};

export type ProviderModelsResponse = {
  models: string[];
};

export type RunModelSelection = {
  providerId: string;
  model: string;
};

export type ThinkingLevel = "auto" | "max" | "xhigh" | "high" | "medium" | "low" | "none";

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

export type SystemUsage = {
  session: {
    id: string;
    requests: number;
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
    turn_count: number;
    context_prompt_tokens: number;
    context_window_tokens: number;
    context_token_ratio: number;
    tool_calls: number;
  };
  process: {
    pid: number;
    uptime_seconds: number;
    rss_bytes?: number | null;
    cpu_percent: number;
  };
  runtime: {
    active_run: boolean;
    terminal_count: number;
  };
};
