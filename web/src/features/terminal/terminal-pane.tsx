import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { useEffect, useRef, useState } from "react";
import { createTerminalOptions } from "./terminal-options";
import "./terminal-pane.css";

/**
 * 渲染与指定终端会话相连的 xterm 实例。
 *
 * @param props 终端会话标识
 * @returns xterm 终端界面
 */
export function TerminalPane({ terminalId }: { terminalId: string }) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    let disposed = false;
    let socket: WebSocket | null = null;
    const terminal = new Terminal(createTerminalOptions());
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(container);
    fit.fit();
    const start = async () => {
      if (disposed) return;
      const protocol = location.protocol === "https:" ? "wss:" : "ws:";
      socket = new WebSocket(`${protocol}//${location.host}/api/terminals/${terminalId}/socket`);
      socket.binaryType = "arraybuffer";
      socket.onmessage = (event) => {
        if (event.data instanceof ArrayBuffer) terminal.write(new Uint8Array(event.data));
        else terminal.write(String(event.data));
      };
      socket.onerror = () => setError("终端连接失败");
      terminal.onData((data) => { if (socket?.readyState === WebSocket.OPEN) socket.send(new TextEncoder().encode(data)); });
      socket.onopen = () => socket?.send(JSON.stringify({ type: "resize", cols: terminal.cols, rows: terminal.rows }));
    };
    void start().catch((reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)));
    const observer = new ResizeObserver(() => {
      fit.fit();
      if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify({ type: "resize", cols: terminal.cols, rows: terminal.rows }));
    });
    observer.observe(container);
    return () => {
      disposed = true;
      observer.disconnect();
      socket?.close();
      terminal.dispose();
    };
  }, [terminalId]);
  return <section className="terminal-pane"><div className="terminal-surface" ref={containerRef} />{error && <div className="pane-error terminal-error">{error}</div>}</section>;
}
