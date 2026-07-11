import { Cable, CalendarClock, Code2 } from "lucide-react";
import { NavLink, Outlet } from "react-router-dom";
import { MOBILE_SIDEBAR_TOGGLE_EVENT } from "../features/workspace/mobile-workbench-state";
import { MiyuLogo } from "../shared/ui/miyu-logo";
import "./app-shell.css";

const navigation = [
  { to: "/", label: "编程", icon: Code2 },
  { to: "/gateways", label: "网关", icon: Cable },
  { to: "/cron-jobs", label: "定时任务", icon: CalendarClock }
];

/**
 * 渲染顶部导航外壳与路由出口。
 *
 * @returns 应用外壳布局
 */
export function AppShell() {
  /** 在移动端请求切换会话侧栏。 */
  const toggleSessionSidebar = () => window.dispatchEvent(new Event(MOBILE_SIDEBAR_TOGGLE_EVENT));

  return (
    <div className="app-shell">
      <header className="topbar">
        <button type="button" className="brand" onClick={toggleSessionSidebar} aria-label="切换会话侧栏">
          <span className="brand-mark"><MiyuLogo size={22} /></span>
          <span className="brand-name">Miyu</span>
          <span className="brand-surface">Web</span>
        </button>
        <div className="topbar-primary" />
        <div className="topbar-actions">
          <nav className="topnav" aria-label="主导航">
            {navigation.map(({ to, label, icon: Icon }) => (
              <NavLink key={to} to={to} end={to === "/"} aria-label={label} className={({ isActive }) => isActive ? "nav-link active" : "nav-link"}>
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
