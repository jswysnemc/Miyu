import { Brain, ChevronDown } from "lucide-react";
import { useState } from "react";

export function ReasoningBlock({ source, live }: { source: string; live?: boolean }) {
  const [open, setOpen] = useState(Boolean(live));
  if (!source) return null;
  return (
    <section className="reasoning-block">
      <button type="button" onClick={() => setOpen((value) => !value)}>
        <Brain size={14} />
        <span>{live ? "正在思考" : "思考过程"}</span>
        <ChevronDown size={14} className={open ? "rotate" : ""} />
      </button>
      {open && <pre>{source}</pre>}
    </section>
  );
}
