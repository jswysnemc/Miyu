import { useMutation, useQuery } from "@tanstack/react-query";
import { Check, ChevronDown, FolderGit2, FolderOpen, X } from "lucide-react";
import { useRef, useState } from "react";
import { api } from "../../api/client";
import { useOutsidePointerDown } from "../../shared/hooks/use-outside-pointer-down";
import { useConfirm } from "../../shared/ui/dialog/dialog-provider";
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
  const rootRef = useRef<HTMLDivElement>(null);
  const confirm = useConfirm();
  const workspaces = useQuery({ queryKey: ["workspaces"], queryFn: api.workspaces.list });
  const active = workspaces.data?.workspaces.find((workspace) => workspace.id === workspaces.data.active_id);
  const switchWorkspace = useMutation({
    mutationFn: (id: string) => switchWithTerminalConfirm(id, confirm),
    onSuccess: (switched) => { if (switched) window.location.reload(); }
  });
  useOutsidePointerDown(rootRef, () => setOpen(false), open);

  /** 登记服务端目录并切换工作区。 */
  const openDirectory = async (path: string) => {
    const workspace = await api.workspaces.add(path);
    const switched = await switchWithTerminalConfirm(workspace.id, confirm);
    if (switched) window.location.reload();
  };

  return (
    <div className="workspace-switcher" ref={rootRef}>
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

/**
 * 切换工作区，遇到终端占用冲突时经确认后关闭终端重试。
 *
 * @param id 目标工作区 ID
 * @param confirm 全局确认对话框方法
 * @returns 是否完成切换
 */
async function switchWithTerminalConfirm(
  id: string,
  confirm: (options: { title: string; description: string; confirmLabel?: string; danger?: boolean }) => Promise<boolean>
): Promise<boolean> {
  try {
    // 1. 先尝试普通切换
    await api.workspaces.switch(id);
    return true;
  } catch (error) {
    // 2. 非终端占用错误直接抛出
    const message = error instanceof Error ? error.message : String(error);
    if (!message.includes("terminal")) throw error;
    // 3. 询问用户是否关闭全部终端并切换
    const confirmed = await confirm({
      title: "关闭终端并切换工作区",
      description: "当前有终端会话在运行，关闭全部终端并切换？",
      confirmLabel: "关闭并切换",
      danger: true
    });
    if (!confirmed) return false;
    // 4. 携带 close_terminals=true 重试
    await api.workspaces.switch(id, true);
    return true;
  }
}
