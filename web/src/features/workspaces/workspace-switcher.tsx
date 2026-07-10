import { useMutation, useQuery } from "@tanstack/react-query";
import { Check, ChevronDown, FolderGit2, FolderOpen, X } from "lucide-react";
import { useState } from "react";
import { api } from "../../api/client";
import { ServerDirectoryDialog } from "./server-directory-dialog";
import "./workspace-switcher.css";

/**
 * 渲染紧凑工作区入口、最近工作区和服务端目录浏览器。
 *
 * @returns 工作区选择器
 */
export function WorkspaceSwitcher() {
  const [open, setOpen] = useState(false);
  const [browserOpen, setBrowserOpen] = useState(false);
  const workspaces = useQuery({ queryKey: ["workspaces"], queryFn: api.workspaces.list });
  const active = workspaces.data?.workspaces.find((workspace) => workspace.id === workspaces.data.active_id);
  const switchWorkspace = useMutation({ mutationFn: api.workspaces.switch, onSuccess: () => window.location.reload() });

  /** 登记服务端目录并切换工作区。 */
  const openDirectory = async (path: string) => {
    const workspace = await api.workspaces.add(path);
    await api.workspaces.switch(workspace.id);
    window.location.reload();
  };

  return (
    <div className="workspace-switcher">
      <button className="workspace-trigger" type="button" onClick={() => setOpen((value) => !value)} aria-expanded={open}>
        <FolderGit2 size={15} /><strong>{active?.name ?? "工作区"}</strong><ChevronDown size={13} className={open ? "open" : ""} />
      </button>
      {open && (
        <div className="workspace-menu">
          <div className="workspace-menu-head"><span><strong>{active?.name}</strong><small>{active?.path}</small></span><button type="button" aria-label="关闭工作区菜单" onClick={() => setOpen(false)}><X size={15} /></button></div>
          <div className="workspace-items">
            {workspaces.data?.workspaces.map((workspace) => (
              <button type="button" className="workspace-item" key={workspace.id} onClick={() => workspace.id !== workspaces.data?.active_id && switchWorkspace.mutate(workspace.id)}>
                <span><strong>{workspace.name}</strong><small>{workspace.path}</small></span>{workspace.id === workspaces.data?.active_id && <Check size={14} />}
              </button>
            ))}
          </div>
          <button type="button" className="workspace-add" onClick={() => { setOpen(false); setBrowserOpen(true); }}><FolderOpen size={15} /><span>浏览服务端目录</span></button>
          {switchWorkspace.error && <p className="form-error workspace-error">{switchWorkspace.error.message}</p>}
        </div>
      )}
      <ServerDirectoryDialog open={browserOpen} onClose={() => setBrowserOpen(false)} onSelect={openDirectory} />
    </div>
  );
}
