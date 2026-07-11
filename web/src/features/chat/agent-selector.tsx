import { Bot } from "lucide-react";
import type { AgentChoice } from "../agents/agent-types";
import { Select } from "../../shared/ui/select/select";
import "./agent-selector.css";

type AgentSelectorProps = {
  choices: AgentChoice[];
  selection: AgentChoice | null;
  loading: boolean;
  disabled: boolean;
  onSelect: (id: string) => void;
};

/**
 * 渲染主界面 Agent 选择器。
 *
 * @param props Agent 选项、当前选择、加载状态和更新回调
 * @returns Agent 单选控件
 */
export function AgentSelector({ choices, selection, loading, disabled, onSelect }: AgentSelectorProps) {
  return (
    <div className="agent-selector">
      <Bot size={13} />
      <Select
        value={selection?.id ?? ""}
        options={choices.map((choice) => ({ value: choice.id, label: choice.name }))}
        disabled={disabled || loading || choices.length === 0}
        ariaLabel="选择 Agent"
        onChange={onSelect}
      />
    </div>
  );
}
