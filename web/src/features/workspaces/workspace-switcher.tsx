import { useMutation, useQuery } from "@tanstack/react-query";
import { Check, ChevronsUpDown, FolderOpen, LoaderCircle, X } from "lucide-react";
import { useState } from "react";
import { api } from "../../api/client";
import "./workspace-switcher.css";

/**
 * 渲染工作区切换菜单和系统文件夹选择入口。
 *
 * @returns 工作区选择器
 */
export function WorkspaceSwitcher() {
  const [open, setOpen] = useState(false);
  const workspaces = useQuery({ queryKey: ["workspaces"], queryFn: api.workspaces.list });
  const active = workspaces.data?.workspaces.find((workspace) => workspace.id === workspaces.data.active_id);

  const switchWorkspace = useMutation({
    mutationFn: api.workspaces.switch,
    onSuccess: () => window.location.reload()
  });
  const pickWorkspace = useMutation({
    mutationFn: api.workspaces.pick,
    onSuccess: (workspace) => {
      if (workspace) window.location.reload();
    }
  });

  const error = switchWorkspace.error ?? pickWorkspace.error;
  return (
    <div className="workspace-switcher">
      <button className="workspace-trigger" type="button" onClick={() => setOpen((value) => !value)} aria-expanded={open}>
        <span className="workspace-copy"><strong>{active?.name ?? "读取工作区"}</strong><small>{active?.path ?? ""}</small></span>
        <ChevronsUpDown size={15} />
      </button>
      {open && (
        <div className="workspace-menu">
          <div className="workspace-menu-head"><span>工作区</span><button type="button" className="icon-button" aria-label="关闭工作区菜单" onClick={() => setOpen(false)}><X size={15} /></button></div>
          <div className="workspace-items">
            {workspaces.data?.workspaces.map((workspace) => (
              <button type="button" className="workspace-item" key={workspace.id} onClick={() => workspace.id !== workspaces.data?.active_id && switchWorkspace.mutate(workspace.id)}>
                <span><strong>{workspace.name}</strong><small>{workspace.path}</small></span>
                {workspace.id === workspaces.data?.active_id && <Check size={15} />}
              </button>
            ))}
          </div>
          <button type="button" className="workspace-add" onClick={() => pickWorkspace.mutate()} disabled={pickWorkspace.isPending}>
            {pickWorkspace.isPending ? <LoaderCircle size={15} className="spin" /> : <FolderOpen size={15} />}
            <span>{pickWorkspace.isPending ? "等待选择文件夹" : "打开文件夹作为工作区"}</span>
          </button>
          <p className="workspace-picker-note">系统选择器会直接登记并切换所选目录</p>
          {error && <p className="form-error workspace-error">{error.message}</p>}
        </div>
      )}
    </div>
  );
}
