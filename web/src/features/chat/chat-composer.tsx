import { ArrowRight, AtSign, Bot, BriefcaseBusiness, GitBranch, ListTree, Paperclip, Square, SquareTerminal } from "lucide-react";
import { useRef } from "react";
import type { ChangeEvent, FormEvent } from "react";
import type { RunModelSelection, ThinkingLevel } from "../../api/contracts";
import type { ChatModelChoice } from "./chat-model-options";
import { AttachmentStrip } from "./composer/attachment-strip";
import { ComposerTextarea } from "./composer/composer-textarea";
import type { ComposerTextareaHandle } from "./composer/composer-textarea";
import type { ComposerAttachment } from "./composer/use-composer-attachments";
import { resolveComposerAvailability } from "./composer-availability";
import { ModelThinkingSelector } from "./model-thinking-selector";
import type { LiveRunState } from "./run-event-reducer";
import type { AgentChoice } from "../agents/agent-types";
import { AgentSelector } from "./agent-selector";
import { WorkspaceSwitcher } from "../workspaces/workspace-switcher";
import { SystemUsage } from "../usage/system-usage";
import { useQuery } from "@tanstack/react-query";
import { api } from "../../api/client";
import { TodoMarkdownView } from "../todo/todo-markdown-view";
import { useRuntimeActivity } from "../runtime-activity/use-runtime-activity";
import "./chat-composer.css";

type ChatComposerProps = {
  value: string;
  mode: "plan" | "yolo";
  attachments: ComposerAttachment[];
  historyEntries: string[];
  thinkingLevel: ThinkingLevel;
  choices: ChatModelChoice[];
  selection: ChatModelChoice | null;
  modelLoading: boolean;
  running: boolean;
  runStatus: LiveRunState["status"];
  sessionAvailable: boolean;
  agentChoices: AgentChoice[];
  agentSelection: AgentChoice | null;
  agentLoading: boolean;
  sessionId?: string;
  onChange: (value: string) => void;
  onModeChange: (mode: "plan" | "yolo") => void;
  onThinkingLevelChange: (level: ThinkingLevel) => void;
  onAddImages: (files: File[], selectionStart: number, selectionEnd: number) => Promise<number | undefined>;
  onRemoveAttachment: (id: number) => void;
  onModelSelect: (selection: RunModelSelection) => void;
  onSubmit: () => void;
  onStop: () => void;
  onAgentSelect: (id: string) => void;
};

/**
 * 渲染 sai-chat 风格的底部输入区。
 *
 * @param props 输入状态、模型状态、附件状态和操作回调
 * @returns 聊天输入区
 */
export function ChatComposer(props: ChatComposerProps) {
  const git = useQuery({ queryKey:["workspace-diff"], queryFn:api.workspace.diff, staleTime:20_000 });
  const runtimeActivity = useRuntimeActivity();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const textareaRef = useRef<ComposerTextareaHandle>(null);

  /**
   * 提交当前输入内容。
   *
   * @param event 表单提交事件
   */
  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    if (availability.sendDisabled) return;
    props.onSubmit();
  };

  /**
   * 读取文件选择器中的全部图片。
   *
   * @param event 文件输入变更事件
   */
  const handleFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(event.target.files ?? []);
    event.target.value = "";
    if (files.length === 0) return;
    void props.onAddImages(files, props.value.length, props.value.length);
  };

  const availability = resolveComposerAvailability({
    sessionAvailable: props.sessionAvailable,
    runActive: props.running,
    runStatus: props.runStatus,
    hasDraft: Boolean(props.value.trim()) || props.attachments.length > 0
  });
  return (
    <div className="composer-shell">
      <form className="composer" onSubmit={handleSubmit}>
        <div className="composer-context-bar">
          <WorkspaceSwitcher />
          {git.data?.repository && <span className="composer-context-chip" title={git.data.branch}><GitBranch size={13}/><span>{git.data.branch || "Git"}</span></span>}
          <SystemUsage />
          <AgentSelector choices={props.agentChoices} selection={props.agentSelection} loading={props.agentLoading} disabled={props.running} onSelect={props.onAgentSelect} />
          <TodoMarkdownView sessionId={props.sessionId} compact />
          <div className="composer-mode" aria-label="运行模式">
            <button type="button" className={props.mode === "yolo" ? "active" : ""} onClick={() => props.onModeChange("yolo")} disabled={props.running} title="工作模式"><BriefcaseBusiness size={13} /><span>工作</span></button>
            <button type="button" className={props.mode === "plan" ? "active" : ""} onClick={() => props.onModeChange("plan")} disabled={props.running} title="规划模式"><ListTree size={13} /><span>规划</span></button>
          </div>
          <button type="button" className={`composer-rail-button composer-activity-button${runtimeActivity.runningTasks > 0 ? " is-active" : ""}`} onClick={() => window.dispatchEvent(new Event("miyu:toggle-terminal"))} title={runtimeActivity.runningTasks > 0 ? `${runtimeActivity.runningTasks} 个后台任务进行中` : "打开终端和后台管理"} aria-label="打开终端和后台管理">
            <SquareTerminal size={14} />
            {runtimeActivity.runningTasks > 0 && <span className="composer-activity-badge">{runtimeActivity.runningTasks}</span>}
          </button>
          {runtimeActivity.runningSubagents > 0 && (
            <button type="button" className="composer-rail-button composer-activity-button is-active" onClick={() => window.dispatchEvent(new Event("miyu:open-subagents"))} title={`${runtimeActivity.runningSubagents} 个子智能体运行中`} aria-label="查看子智能体">
              <Bot size={14} />
              <span className="composer-activity-badge">{runtimeActivity.runningSubagents}</span>
            </button>
          )}
        </div>
        <AttachmentStrip attachments={props.attachments} onRemove={props.onRemoveAttachment} />
        <ComposerTextarea
          ref={textareaRef}
          value={props.value}
          historyEntries={props.historyEntries}
          disabled={availability.inputDisabled}
          placeholder={props.sessionAvailable ? "输入消息，Enter 发送" : "请先选择会话"}
          onChange={props.onChange}
          onPasteImages={props.onAddImages}
          onSubmit={() => {
            if (!availability.sendDisabled) props.onSubmit();
          }}
        />
        <div className="composer-footer">
          <div className="composer-toolrail">
            <div className="composer-model-group">
              <ModelThinkingSelector
                choices={props.choices}
                selection={props.selection}
                thinkingLevel={props.thinkingLevel}
                loading={props.modelLoading}
                disabled={props.running}
                onModelSelect={props.onModelSelect}
                onThinkingLevelChange={props.onThinkingLevelChange}
              />
            </div>
            <button type="button" className="composer-rail-button" onClick={() => textareaRef.current?.openMentionPicker()} disabled={availability.inputDisabled} title="插入引用" aria-label="插入引用"><AtSign size={14} /></button>
          </div>
          <div className="composer-actions">
            <input ref={fileInputRef} type="file" accept="image/*" multiple onChange={handleFileChange} hidden />
            <button type="button" className="composer-icon-button" onClick={() => fileInputRef.current?.click()} disabled={availability.inputDisabled} aria-label="添加图片"><Paperclip size={18} /></button>
            {availability.showStop && (
              <button type="button" className="composer-send stop" onClick={props.onStop} aria-label="停止运行"><Square size={13} fill="currentColor" /></button>
            )}
            <button type="submit" className="composer-send" disabled={availability.sendDisabled} aria-label={availability.showStop ? "加入会话队列" : "发送消息"}><ArrowRight size={18} /></button>
          </div>
        </div>
      </form>
    </div>
  );
}
