import { Select } from "../../../shared/ui/select/select";
import type { ChatModelChoice } from "../../chat/chat-model-options";

type SubagentModelPickerProps = {
  choices: ChatModelChoice[];
  providerId: string;
  model: string;
  onChange: (providerId: string, model: string) => void;
};

const INHERIT_VALUE = "__inherit__";
const SEPARATOR = "\t";

/**
 * 渲染子智能体统一模型选择器,可选择沿用主对话模型。
 *
 * @param props 模型选项、当前供应商与模型、变化回调
 * @returns 子智能体模型选择区
 */
export function SubagentModelPicker({ choices, providerId, model, onChange }: SubagentModelPickerProps) {
  const options = [
    { value: INHERIT_VALUE, label: "沿用主对话模型" },
    ...choices.map((choice) => ({
      value: `${choice.providerId}${SEPARATOR}${choice.model}`,
      label: `${choice.providerName} / ${choice.model}`
    }))
  ];
  const current = providerId && model ? `${providerId}${SEPARATOR}${model}` : INHERIT_VALUE;

  /**
   * 解析选择值并回传供应商与模型。
   *
   * @param value 编码后的选项值
   */
  const handleChange = (value: string) => {
    if (value === INHERIT_VALUE) {
      onChange("", "");
      return;
    }
    const [nextProvider, nextModel] = value.split(SEPARATOR);
    onChange(nextProvider ?? "", nextModel ?? "");
  };

  return (
    <Select
      value={current}
      options={options}
      ariaLabel="选择子智能体模型"
      menuPreferredWidth={280}
      menuMinimumWidth={220}
      onChange={handleChange}
    />
  );
}
