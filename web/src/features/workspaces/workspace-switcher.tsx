import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, ChevronsUpDown, FolderPlus, X } from "lucide-react";
import { useState } from "react";
import { api } from "../../api/client";
import "./workspace-switcher.css";

export function WorkspaceSwitcher() {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [adding, setAdding] = useState(false);
  const [path, setPath] = useState("");
  const workspaces = useQuery({ queryKey: ["workspaces"], queryFn: api.workspaces.list });
  const active = workspaces.data?.workspaces.find((workspace) => workspace.id === workspaces.data.active_id);

  const switchWorkspace = useMutation({
    mutationFn: api.workspaces.switch,
    onSuccess: () => window.location.reload()
  });
  const addWorkspace = useMutation({
    mutationFn: (value: string) => api.workspaces.add(value),
    onSuccess: async (workspace) => {
      await queryClient.invalidateQueries({ queryKey: ["workspaces"] });
      setPath("");
      setAdding(false);
      await switchWorkspace.mutateAsync(workspace.id);
    }
  });

  return (
    <div className="workspace-switcher">
      <button className="workspace-trigger" type="button" onClick={() => setOpen((value) => !value)} aria-expanded={open}>
        <span className="workspace-copy">
          <strong>{active?.name ?? "读取工作区"}</strong>
          <small>{active?.path ?? ""}</small>
        </span>
        <ChevronsUpDown size={15} />
      </button>
      {open && (
        <div className="workspace-menu">
          <div className="workspace-menu-head">
            <span>工作区</span>
            <button type="button" className="icon-button" aria-label="关闭工作区菜单" onClick={() => setOpen(false)}><X size={15} /></button>
          </div>
          <div className="workspace-items">
            {workspaces.data?.workspaces.map((workspace) => (
              <button
                type="button"
                className="workspace-item"
                key={workspace.id}
                onClick={() => workspace.id !== workspaces.data?.active_id && switchWorkspace.mutate(workspace.id)}
              >
                <span>
                  <strong>{workspace.name}</strong>
                  <small>{workspace.path}</small>
                </span>
                {workspace.id === workspaces.data?.active_id && <Check size={15} />}
              </button>
            ))}
          </div>
          {adding ? (
            <form className="workspace-add-form" onSubmit={(event) => { event.preventDefault(); if (path.trim()) addWorkspace.mutate(path.trim()); }}>
              <label htmlFor="workspace-path">目录绝对路径</label>
              <input id="workspace-path" value={path} onChange={(event) => setPath(event.target.value)} placeholder="/home/user/workspace/project" autoFocus />
              <div className="workspace-form-actions">
                <button type="button" className="button secondary" onClick={() => setAdding(false)}>取消</button>
                <button type="submit" className="button primary" disabled={addWorkspace.isPending}>添加并切换</button>
              </div>
              {addWorkspace.error && <p className="form-error">{addWorkspace.error.message}</p>}
            </form>
          ) : (
            <button type="button" className="workspace-add" onClick={() => setAdding(true)}>
              <FolderPlus size={15} /> 添加工作区
            </button>
          )}
          {switchWorkspace.error && <p className="form-error workspace-error">{switchWorkspace.error.message}</p>}
        </div>
      )}
    </div>
  );
}
