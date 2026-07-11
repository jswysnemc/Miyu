import { ArrowRight, AtSign, BriefcaseBusiness, Eraser, ListTree, Paperclip, Square } from "lucide-react";
import { useRef } from "react";
import type { ChangeEvent, FormEvent } from "react";
import type { RunModelSelection, ThinkingLevel } from "../../api/contracts";
import type { ChatModelChoice } from "./chat-model-options";
import { AttachmentStrip } from "./composer/attachment-strip";
import { ComposerTextarea } from "./composer/composer-textarea";
import type { ComposerTextareaHandle } from "./composer/composer-textarea";
import type { ComposerAttachment } from "./composer/use-composer-attachments";
import { resolveComposerAvailability } from "./composer-availability";
import { ModelSelector } from "./model-selector";
import type { LiveRunState } from "./run-event-reducer";
import { ThinkingSelector } from "./thinking-selector";
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
  onChange: (value: string) => void;
  onModeChange: (mode: "plan" | "yolo") => void;
  onThinkingLevelChange: (level: ThinkingLevel) => void;
  onAddImages: (files: File[], selectionStart: number, selectionEnd: number) => Promise<number | undefined>;
  onRemoveAttachment: (id: number) => void;
  onModelSelect: (selection: RunModelSelection) => void;
  onSubmit: () => void;
  onStop: () => void;
};

/**
 * 渲染 sai-chat 风格的底部输入区。
 *
 * @param props 输入状态、模型状态、附件状态和操作回调
 * @returns 聊天输入区
 */
export function ChatComposer(props: ChatComposerProps) {
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
              <ModelSelector
                choices={props.choices}
                selection={props.selection}
                loading={props.modelLoading}
                disabled={props.running}
                onSelect={props.onModelSelect}
              />
              <ThinkingSelector
                value={props.thinkingLevel}
                disabled={props.running}
                onChange={props.onThinkingLevelChange}
              />
            </div>
            <div className="composer-mode" aria-label="运行模式">
              <button type="button" className={props.mode === "yolo" ? "active" : ""} onClick={() => props.onModeChange("yolo")} disabled={props.running} title="工作模式"><BriefcaseBusiness size={13} /><span>工作</span></button>
              <button type="button" className={props.mode === "plan" ? "active" : ""} onClick={() => props.onModeChange("plan")} disabled={props.running} title="规划模式"><ListTree size={13} /><span>规划</span></button>
            </div>
            <button type="button" className="composer-rail-button" onClick={() => textareaRef.current?.openMentionPicker()} disabled={availability.inputDisabled} title="插入引用" aria-label="插入引用"><AtSign size={14} /></button>
            <button type="button" className="composer-rail-button" onClick={() => { props.onChange(""); for (const attachment of props.attachments) props.onRemoveAttachment(attachment.id); }} disabled={availability.inputDisabled || (!props.value && props.attachments.length === 0)} title="清空输入" aria-label="清空输入"><Eraser size={14} /></button>
          </div>
          <div className="composer-actions">
            <input ref={fileInputRef} type="file" accept="image/*" multiple onChange={handleFileChange} hidden />
            <button type="button" className="composer-icon-button" onClick={() => fileInputRef.current?.click()} disabled={availability.inputDisabled} aria-label="添加图片"><Paperclip size={18} /></button>
            {availability.showStop ? (
              <button type="button" className="composer-send stop" onClick={props.onStop} aria-label="停止运行"><Square size={13} fill="currentColor" /></button>
            ) : (
              <button type="submit" className="composer-send" disabled={availability.sendDisabled} aria-label="发送消息"><ArrowRight size={18} /></button>
            )}
          </div>
        </div>
      </form>
    </div>
  );
}
