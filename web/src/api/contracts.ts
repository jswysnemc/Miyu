export type RunMode = "plan" | "audited" | "yolo";

export type PermissionConfig = {
  default_mode: RunMode;
};

export type PermissionAuditEvent = {
  timestamp_ms: number;
  session_id: string;
  mode: RunMode;
  tool: string;
  decision: "requested" | "approved" | "allowed" | "denied" | "completed" | "failed";
  arguments: Record<string, unknown>;
  detail?: string | null;
};

export type PermissionRequest = {
  id: string;
  session_id: string;
  tool: string;
  arguments: string;
};

export type PermissionDecision =
  | { decision: "allow" }
  | { decision: "deny"; reply?: string | null };

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

export type DirectoryEntry = {
  name: string;
  path: string;
  git_repository: boolean;
};

export type DirectoryListing = {
  current: string;
  parent?: string | null;
  roots: DirectoryEntry[];
  entries: DirectoryEntry[];
};

export type Session = {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  active: boolean;
};

export type WorkspaceSessions = {
  workspace_id: string;
  workspace_name: string;
  workspace_path: string;
  active: boolean;
  sessions: Session[];
};

export type UndoSessionResult = {
  removed: number;
  prompt?: string | null;
  worktree_restored: boolean;
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
  permission?: PermissionDecision | null;
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

export type WeixinLoginPhase =
  | "waiting"
  | "scanned"
  | "need_verify_code"
  | "confirmed"
  | "expired"
  | "failed";

export type WeixinLoginAccount = {
  account_id: string;
  base_url: string;
  cdn_base_url: string;
  user_id?: string | null;
};

export type WeixinLoginSnapshot = {
  session_id: string;
  phase: WeixinLoginPhase;
  qrcode_content: string;
  qrcode_svg: string;
  message?: string | null;
  account?: WeixinLoginAccount | null;
};

export type CronJob = {
  id: string;
  name: string;
  prompt: string;
  session_id: string;
  interval_seconds?: number | null;
  next_run_at: number;
  enabled: boolean;
  failure_count: number;
  last_error?: string | null;
};

export type CreateCronJobRequest = {
  name: string;
  prompt: string;
  session_id: string;
  run_at: number;
  interval_seconds?: number | null;
};

export type UpdateCronJobRequest = {
  enabled: boolean;
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
  web_search_tool_mode?: "hide_builtin" | "rename_local";
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
  permission?: PermissionConfig;
  gateways: GatewayConfig;
  agents?: AgentProfileConfig[];
  default_agent?: string | null;
  subagent?: SubagentConfig;
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

export type AgentProfileConfig = {
  id: string;
  name: string;
  system_prompt?: string;
  enabled_tools?: string[];
  skills_full?: string[];
  skills_named?: string[];
};

export type SubagentConfig = {
  provider_id?: string;
  model?: string;
};

export type ConfigResponse = {
  config: AppConfig;
  secret_sentinel: string;
};

export type ProviderModelsResponse = {
  models: string[];
  metadata: Record<string, { provider:string; context_chars?:number | null; tags?:string[] }>;
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
  input?: string;
  image_urls?: string[];
  status?: "queued" | "running" | "completed" | "interrupted" | "failed";
  discard_user_turn?: boolean;
  restore_input?: string | null;
};

export type ActiveRunsResponse = {
  run?: RunInfo | null;
  runs: RunInfo[];
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

export type BackgroundTask = {
  id: string;
  label: string;
  command: string;
  cwd: string;
  pid: number;
  status: string;
  started_at: number;
  updated_at: number;
  timeout_seconds: number;
};

export type BackgroundTaskOutput = {
  task: BackgroundTask;
  stdout?: string | null;
  stderr?: string | null;
  stdout_truncated: boolean;
  stderr_truncated: boolean;
  tail_lines: number;
};

export type TodoStatus = "pending" | "in_progress" | "completed" | "cancelled";
export type TodoItem = { id:string; text:string; status:TodoStatus; created_at:string; updated_at:string };

export type Subagent = {
  id:string; description:string; subagent_type:string; status:string; max_steps:number;
  started_at:number; updated_at:number; step:number; phase?:string; last_tool?:string;
  result?:string; error?:string; stats?:Record<string, unknown>;
};

export type SubagentTimelineEntry =
  | { kind:"tool"; step:number; name:string; args_preview:string; ok?:boolean|null; output_preview?:string|null }
  | { kind:"text"; text:string }
  | { kind:"reasoning"; text:string };

export type SubagentDetail = Subagent & { timeline: SubagentTimelineEntry[] };

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
    checkpoint_count: number;
    compacted_turns: number;
    latest_checkpoint_at?: string | null;
    latest_checkpoint_reason?: "auto" | "manual" | "legacy" | null;
    compaction_warning?: string | null;
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
