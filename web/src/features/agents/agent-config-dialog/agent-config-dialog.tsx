import { useEffect, useState } from "react";
import { Modal } from "../../../shared/ui/dialog/modal";
import { buildChatModelChoices } from "../../chat/chat-model-options";
import { buildAgentChoices } from "../agent-options";
import { DefaultAgentPicker } from "./default-agent-picker";
import { SubagentModelPicker } from "./subagent-model-picker";
import { readAgentConfigDraft, useAgentConfig, type AgentConfigDraft } from "./use-agent-config";
import "./agent-config-dialog.css";

type AgentConfigDialogProps = {
  open: boolean;
  onClose: () => void;
};

/**
 * 渲染 Agent 配置弹窗:设置全局默认 Agent 与子智能体统一模型。
 *
 * @param props 弹窗开合状态与关闭回调
 * @returns Agent 配置弹窗
 */
export function AgentConfigDialog({ open, onClose }: AgentConfigDialogProps) {
  const { config, isLoading, error, save, saving, saveError } = useAgentConfig();
  const [draft, setDraft] = useState<AgentConfigDraft | null>(null);

  useEffect(() => {
    if (open && config) setDraft(readAgentConfigDraft(config));
  }, [open, config]);

  const agentChoices = config ? buildAgentChoices(config) : [];
  const modelChoices = config ? buildChatModelChoices(config) : [];

  /** 保存草稿并关闭弹窗。 */
  const handleSave = async () => {
    if (!draft) return;
    await save(draft);
    onClose();
  };

  return (
    <Modal
      open={open}
      title="Agent 配置"
      description="设置默认使用的 Agent,以及子智能体统一使用的模型。"
      size="medium"
      onClose={onClose}
      footer={
        <div className="agent-config-footer">
          {(error || saveError) && <span className="agent-config-error">{(saveError ?? error)?.message}</span>}
          <button type="button" className="agent-config-cancel" onClick={onClose}>取消</button>
          <button type="button" className="agent-config-save" onClick={() => void handleSave()} disabled={!draft || saving}>
            {saving ? "保存中" : "保存"}
          </button>
        </div>
      }
    >
      {isLoading || !draft ? (
        <p className="agent-config-loading">正在读取配置</p>
      ) : (
        <div className="agent-config-body">
          <section className="agent-config-section">
            <h3>默认 Agent</h3>
            <p>新会话与未指定 Agent 的运行默认采用它。</p>
            <DefaultAgentPicker
              choices={agentChoices}
              value={draft.defaultAgent}
              onChange={(id) => setDraft({ ...draft, defaultAgent: id })}
            />
          </section>
          <section className="agent-config-section">
            <h3>子智能体模型</h3>
            <p>task 工具启动的子智能体统一使用该模型,留空则沿用主对话。</p>
            <SubagentModelPicker
              choices={modelChoices}
              providerId={draft.subagentProviderId}
              model={draft.subagentModel}
              onChange={(providerId, model) => setDraft({ ...draft, subagentProviderId: providerId, subagentModel: model })}
            />
          </section>
        </div>
      )}
    </Modal>
  );
}
