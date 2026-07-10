import { useMemo, useState } from "react";
import type { AgentSkillOption } from "./agents-types";

type SkillColumnId = "full" | "named" | "off";

type SkillAssignmentBoardProps = {
  /** 可选 skill 列表（名称与描述） */
  skills: AgentSkillOption[];
  /** 完整启用的 skill 名称集合 */
  fullNames: string[];
  /** 半启用（仅暴露名称）的 skill 名称集合 */
  namedNames: string[];
  /** 分配变化回调，参数为新的 skills_full 与 skills_named */
  onChange: (fullNames: string[], namedNames: string[]) => void;
};

type ColumnMeta = {
  id: SkillColumnId;
  title: string;
  hint: string;
};

const COLUMNS: ColumnMeta[] = [
  { id: "full", title: "启用", hint: "加载名称与描述" },
  { id: "named", title: "半启用", hint: "仅暴露名称" },
  { id: "off", title: "不启用", hint: "完全不暴露" }
];

const DRAG_MIME = "application/x-miyu-skill";

/**
 * 三栏拖放板：把 skill 分配到「启用 / 半启用 / 不启用」三列。
 *
 * @param props skill 选项、当前分配和变化回调
 * @returns 三栏拖放板区域
 */
export function SkillAssignmentBoard({ skills, fullNames, namedNames, onChange }: SkillAssignmentBoardProps) {
  const [hoverColumn, setHoverColumn] = useState<SkillColumnId | null>(null);

  /** 合并服务端 skill 列表与配置中残留的未知名称，保证已配置项始终可见。 */
  const allSkills = useMemo(() => {
    const known = new Map(skills.map((skill) => [skill.name, skill]));
    for (const name of [...fullNames, ...namedNames]) {
      if (!known.has(name)) known.set(name, { name, description: "" });
    }
    return [...known.values()];
  }, [skills, fullNames, namedNames]);

  /** 计算某个 skill 当前所在列。 */
  const columnOf = (name: string): SkillColumnId => {
    if (fullNames.includes(name)) return "full";
    if (namedNames.includes(name)) return "named";
    return "off";
  };

  /** 把 skill 移动到目标列并回调新的分配结果。 */
  const moveTo = (name: string, target: SkillColumnId) => {
    if (columnOf(name) === target) return;
    const nextFull = fullNames.filter((item) => item !== name);
    const nextNamed = namedNames.filter((item) => item !== name);
    if (target === "full") nextFull.push(name);
    if (target === "named") nextNamed.push(name);
    onChange(nextFull, nextNamed);
  };

  /** 处理拖拽进入列时的悬停高亮与放置许可。 */
  const handleDragOver = (event: React.DragEvent, column: SkillColumnId) => {
    if (!event.dataTransfer.types.includes(DRAG_MIME)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    setHoverColumn(column);
  };

  /** 处理放置：读取拖拽携带的 skill 名并移动到目标列。 */
  const handleDrop = (event: React.DragEvent, column: SkillColumnId) => {
    event.preventDefault();
    setHoverColumn(null);
    const name = event.dataTransfer.getData(DRAG_MIME);
    if (name) moveTo(name, column);
  };

  return (
    <div className="agent-skill-board">
      {COLUMNS.map((column, columnIndex) => {
        const items = allSkills.filter((skill) => columnOf(skill.name) === column.id);
        return (
          <section
            key={column.id}
            className={`agent-skill-column${hoverColumn === column.id ? " drop-target" : ""}`}
            onDragOver={(event) => handleDragOver(event, column.id)}
            onDragLeave={() => setHoverColumn((current) => (current === column.id ? null : current))}
            onDrop={(event) => handleDrop(event, column.id)}
          >
            <header className="agent-skill-column-head">
              <span className="agent-skill-column-title">{column.title}</span>
              <span className="agent-skill-column-count">{items.length}</span>
              <span className="agent-skill-column-hint">{column.hint}</span>
            </header>
            <div className="agent-skill-column-body">
              {items.length === 0 && <p className="agent-skill-empty">拖动 skill 到此列</p>}
              {items.map((skill) => (
                <article
                  key={skill.name}
                  className="agent-skill-card"
                  draggable
                  onDragStart={(event) => {
                    event.dataTransfer.setData(DRAG_MIME, skill.name);
                    event.dataTransfer.effectAllowed = "move";
                  }}
                >
                  <div className="agent-skill-card-main">
                    <span className="agent-skill-card-name">{skill.name}</span>
                    {skill.description && (
                      <span className={`agent-skill-card-desc${column.id === "off" ? " muted" : ""}`}>
                        {skill.description}
                      </span>
                    )}
                  </div>
                  <div className="agent-skill-card-actions">
                    <button
                      type="button"
                      className="agent-skill-move"
                      disabled={columnIndex === 0}
                      aria-label={`将 ${skill.name} 左移一列`}
                      onClick={() => moveTo(skill.name, COLUMNS[columnIndex - 1].id)}
                    >
                      <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
                        <path d="M10 3 5 8l5 5" fill="none" stroke="currentColor" strokeWidth="1.6" />
                      </svg>
                    </button>
                    <button
                      type="button"
                      className="agent-skill-move"
                      disabled={columnIndex === COLUMNS.length - 1}
                      aria-label={`将 ${skill.name} 右移一列`}
                      onClick={() => moveTo(skill.name, COLUMNS[columnIndex + 1].id)}
                    >
                      <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
                        <path d="m6 3 5 5-5 5" fill="none" stroke="currentColor" strokeWidth="1.6" />
                      </svg>
                    </button>
                  </div>
                </article>
              ))}
            </div>
          </section>
        );
      })}
    </div>
  );
}
