import { CheckCheck, Search, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "../../../shared/ui/button/button";
import type { AgentToolOption } from "./agents-types";
import "./agent-permissions.css";

type AgentToolPermissionsProps = {
  /** 全部可用工具 */
  tools: AgentToolOption[];
  /** 当前启用的工具名称 */
  enabled: string[];
  /** 工具权限变化回调 */
  onChange: (enabledTools: string[]) => void;
};

type PermissionCheckboxProps = {
  /** 是否全部选中 */
  checked: boolean;
  /** 是否部分选中 */
  indeterminate: boolean;
  /** 可访问名称 */
  ariaLabel: string;
  /** 选择状态变化回调 */
  onChange: (checked: boolean) => void;
};

/**
 * 批量更新工具启用状态，同时保持原有顺序并移除重复项。
 *
 * @param enabled 当前启用的工具名称
 * @param names 本次需要更新的工具名称
 * @param checked 是否启用
 * @returns 更新后的 enabled_tools 数组
 */
export function updateEnabledTools(enabled: string[], names: string[], checked: boolean): string[] {
  const targetNames = new Set(names);
  const next = enabled.filter((name, index) => enabled.indexOf(name) === index && !targetNames.has(name));
  if (checked) next.push(...names.filter((name, index) => names.indexOf(name) === index));
  return next;
}

/**
 * 渲染支持部分选中状态的权限复选框。
 *
 * @param props 选择状态、可访问名称和变化回调
 * @returns 权限复选框
 */
function PermissionCheckbox({ checked, indeterminate, ariaLabel, onChange }: PermissionCheckboxProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (inputRef.current) inputRef.current.indeterminate = indeterminate;
  }, [indeterminate]);

  return (
    <input
      ref={inputRef}
      type="checkbox"
      checked={checked}
      aria-label={ariaLabel}
      onChange={(event) => onChange(event.target.checked)}
    />
  );
}

/**
 * 渲染可搜索、可分组批量选择的 Agent 工具权限面板。
 *
 * @param props 工具列表、当前启用项和变化回调
 * @returns 工具权限面板
 */
export function AgentToolPermissions({ tools, enabled, onChange }: AgentToolPermissionsProps) {
  const [query, setQuery] = useState("");
  const normalizedQuery = query.trim().toLocaleLowerCase();

  /** 按分组整理工具，并根据查询内容筛选可见分组。 */
  const groups = useMemo(() => {
    const grouped = new Map<string, AgentToolOption[]>();
    for (const tool of tools) {
      const items = grouped.get(tool.group) ?? [];
      items.push(tool);
      grouped.set(tool.group, items);
    }
    return [...grouped.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([group, items]) => ({
        group,
        allItems: items,
        visibleItems: normalizedQuery.length === 0
          ? items
          : items.filter((tool) => (
            tool.name.toLocaleLowerCase().includes(normalizedQuery)
            || group.toLocaleLowerCase().includes(normalizedQuery)
          ))
      }))
      .filter(({ visibleItems }) => visibleItems.length > 0);
  }, [normalizedQuery, tools]);

  if (tools.length === 0) {
    return <p className="agent-permissions-empty">暂无可用工具。</p>;
  }

  const enabledCount = tools.filter((tool) => enabled.includes(tool.name)).length;

  return (
    <div className="agent-permissions-panel agent-tool-permissions">
      <div className="agent-permissions-toolbar">
        <label className="agent-permissions-search">
          <Search size={14} aria-hidden="true" />
          <input
            type="search"
            value={query}
            placeholder="搜索工具或分组"
            aria-label="搜索工具或分组"
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <span className="agent-permissions-summary">已启用 {enabledCount}/{tools.length}</span>
        <div className="agent-permissions-actions">
          <Button onClick={() => onChange(tools.map((tool) => tool.name))}>
            <CheckCheck size={14} aria-hidden="true" />
            全部启用
          </Button>
          <Button onClick={() => onChange([])} disabled={enabled.length === 0}>
            <X size={14} aria-hidden="true" />
            全部清空
          </Button>
        </div>
      </div>

      {groups.length === 0 ? (
        <p className="agent-permissions-empty">没有匹配的工具。</p>
      ) : (
        <div className="agent-tool-permission-groups">
          {groups.map(({ group, allItems, visibleItems }) => {
            const groupNames = allItems.map((tool) => tool.name);
            const checkedCount = groupNames.filter((name) => enabled.includes(name)).length;
            return (
              <section key={group} className="agent-tool-permission-group">
                <header className="agent-tool-permission-group-head">
                  <PermissionCheckbox
                    checked={checkedCount === groupNames.length}
                    indeterminate={checkedCount > 0 && checkedCount < groupNames.length}
                    ariaLabel={`选择${group}分组的全部工具`}
                    onChange={(checked) => onChange(updateEnabledTools(enabled, groupNames, checked))}
                  />
                  <strong>{group}</strong>
                  <span>{checkedCount}/{groupNames.length}</span>
                </header>
                <div className="agent-tool-permission-items">
                  {visibleItems.map((tool) => (
                    <label key={tool.name} className="agent-tool-permission-item">
                      <input
                        type="checkbox"
                        checked={enabled.includes(tool.name)}
                        onChange={(event) => onChange(updateEnabledTools(enabled, [tool.name], event.target.checked))}
                      />
                      <span>{tool.name}</span>
                    </label>
                  ))}
                </div>
              </section>
            );
          })}
        </div>
      )}
    </div>
  );
}
