import { Check } from "lucide-react";
import type { AgentChoice } from "../agent-types";

type DefaultAgentPickerProps = {
  choices: AgentChoice[];
  value: string;
  onChange: (id: string) => void;
};

/**
 * 渲染全局默认 Agent 单选列表。
 *
 * @param props Agent 选项、当前值与变化回调
 * @returns 默认 Agent 选择区
 */
export function DefaultAgentPicker({ choices, value, onChange }: DefaultAgentPickerProps) {
  return (
    <div className="agent-config-picker">
      {choices.map((choice) => (
        <button
          key={choice.id}
          type="button"
          className={`agent-config-option${choice.id === value ? " selected" : ""}`}
          onClick={() => onChange(choice.id)}
        >
          <span className="agent-config-option-name">{choice.name}</span>
          {choice.id === value && <Check size={15} />}
        </button>
      ))}
    </div>
  );
}
