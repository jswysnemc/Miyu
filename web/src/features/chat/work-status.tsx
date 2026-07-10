import { LoaderCircle } from "lucide-react";
import type { LiveRunState } from "./run-event-reducer";

const labels: Record<LiveRunState["status"], string> = {
  idle: "空闲",
  waiting_response: "等待响应",
  thinking: "思考",
  working: "工作"
};

export function WorkStatus({ status }: { status: LiveRunState["status"] }) {
  if (status === "idle") return null;
  return <div className={`work-status ${status}`}><LoaderCircle size={13} className="spin" /> {labels[status]}</div>;
}
