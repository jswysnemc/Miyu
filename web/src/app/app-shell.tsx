import { Cable, Code2 } from "lucide-react";
import { NavLink, Outlet } from "react-router-dom";
import { WorkspaceSwitcher } from "../features/workspaces/workspace-switcher";
import { SystemUsage } from "../features/usage/system-usage";
import { useChatAgentContext } from "../features/agents/chat-agent-context";
import { AgentSelector } from "../features/chat/agent-selector";
import { MiyuLogo } from "../shared/ui/miyu-logo";
import "./app-shell.css";

const navigation = [
  { to: "/", label: "编程", icon: Code2 },
  { to: "/gateways", label: "网关", icon: Cable }
];

/**
 * 渲染顶部导航外壳与路由出口。
 *
 * @returns 应用外壳布局
 */
export function AppShell() {
  const chatAgent = useChatAgentContext();
  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand" aria-label="Miyu Web">
          <span className="brand-mark"><MiyuLogo size={22} /></span>
          <span className="brand-name">Miyu</span>
          <span className="brand-surface">Web</span>
        </div>
        <div className="topbar-primary">
          <WorkspaceSwitcher />
          <SystemUsage />
          <AgentSelector
            choices={chatAgent.choices}
            selection={chatAgent.selection}
            loading={chatAgent.isLoading}
            disabled={false}
            onSelect={chatAgent.selectAgent}
          />
        </div>
        <div className="topbar-actions">
          <nav className="topnav" aria-label="主导航">
            {navigation.map(({ to, label, icon: Icon }) => (
              <NavLink key={to} to={to} end={to === "/"} className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}>
                <Icon size={16} strokeWidth={1.8} /><span>{label}</span>
              </NavLink>
            ))}
          </nav>
        </div>
      </header>
      <main className="app-content">
        <Outlet />
      </main>
    </div>
  );
}
