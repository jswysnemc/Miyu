import { Bot, Cable, Code2, Settings } from "lucide-react";
import { NavLink, Outlet } from "react-router-dom";
import { WorkspaceSwitcher } from "../features/workspaces/workspace-switcher";
import { SystemUsage } from "../features/usage/system-usage";
import "./app-shell.css";

const navigation = [
  { to: "/", label: "编程", icon: Code2 },
  { to: "/gateways", label: "网关", icon: Cable },
  { to: "/settings", label: "配置", icon: Settings }
];

export function AppShell() {
  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand" aria-label="Miyu Web">
          <span className="brand-mark"><Bot size={19} strokeWidth={1.8} /></span>
          <span className="brand-name">Miyu</span>
          <span className="brand-surface">Web</span>
        </div>
        <WorkspaceSwitcher />
        <div className="topbar-actions">
          <SystemUsage />
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
