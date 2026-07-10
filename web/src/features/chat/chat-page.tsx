import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowUp, Square, WandSparkles } from "lucide-react";
import { FormEvent, KeyboardEvent, useCallback, useEffect, useRef, useState } from "react";
import { api } from "../../api/client";
import type { HistoryEntry } from "../../api/contracts";
import { MarkdownRenderer } from "./markdown-renderer";
import { ReasoningBlock } from "./reasoning-block";
import { ToolLifecycleCard } from "./tool-lifecycle-card";
import { useRunStream } from "./use-run-stream";
import { WorkStatus } from "./work-status";
import "./chat-page.css";

/** 渲染当前会话历史、实时运行事件和消息输入区。 */
export function ChatPage() {
  const queryClient = useQueryClient();
  const sessions = useQuery({ queryKey: ["sessions"], queryFn: api.sessions.list });
  const activeSession = sessions.data?.find((session) => session.active);
  const messages = useQuery({
    queryKey: ["messages", activeSession?.id],
    queryFn: () => api.sessions.messages(activeSession!.id),
    enabled: Boolean(activeSession)
  });
  const onSettled = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["sessions"] });
  }, [queryClient]);
  const run = useRunStream(onSettled);
  const [input, setInput] = useState("");
  const [mode, setMode] = useState<"plan" | "yolo">("yolo");
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    run.reset();
  }, [activeSession?.id]);

  useEffect(() => {
    const element = scrollRef.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [messages.data, run.state.content, run.state.reasoning, run.state.tools.length]);

  const submit = async (event?: FormEvent) => {
    event?.preventDefault();
    const value = input.trim();
    if (!value || !activeSession || (run.state.runId && !run.state.completed)) return;
    await queryClient.invalidateQueries({ queryKey: ["messages", activeSession.id] });
    setInput("");
    await run.start(activeSession.id, value, mode).catch((error: unknown) => {
      setInput(value);
      throw error;
    });
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void submit();
    }
  };

  const running = Boolean(run.state.runId && !run.state.completed);
  return (
    <div className="chat-page">
      <header className="chat-header">
        <div>
          <span className="eyebrow">Agent workspace</span>
          <h1>{activeSession?.title ?? "选择会话"}</h1>
        </div>
        <WorkStatus status={run.state.status} />
      </header>
      <div className="message-scroll" ref={scrollRef}>
        <div className="message-column">
          {messages.isLoading && <div className="empty-chat">正在读取会话历史</div>}
          {messages.data?.length === 0 && !run.state.runId && (
            <div className="empty-chat">
              <span className="empty-mark"><WandSparkles size={23} /></span>
              <h2>从工作区开始</h2>
              <p>描述需要完成的代码任务，Miyu 会在当前工作区中分析、调用工具并实时汇报进度。</p>
            </div>
          )}
          {messages.data?.map((message, index) => <HistoryMessage key={`${message.timestamp}-${index}`} message={message} />)}
          {run.state.runId && (
            <>
              <article className="message user-message"><div className="message-content">{run.state.userInput}</div></article>
              <article className="message assistant-message live-message">
                <ReasoningBlock source={run.state.reasoning} live={running} />
                {run.state.content && <MarkdownRenderer source={run.state.content} />}
                <div className="tool-stack">
                  {run.state.tools.map((tool) => <ToolLifecycleCard tool={tool} key={tool.id} />)}
                </div>
                {running && !run.state.content && !run.state.reasoning && run.state.tools.length === 0 && <div className="response-pulse"><span /><span /><span /></div>}
                {run.state.error && <div className="run-error">{run.state.error}</div>}
              </article>
            </>
          )}
          {messages.error && <div className="run-error">{messages.error.message}</div>}
        </div>
      </div>
      <form className="composer" onSubmit={submit}>
        <textarea
          value={input}
          onChange={(event) => setInput(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={activeSession ? "向 Miyu 描述编程任务" : "请先选择会话"}
          disabled={!activeSession || running}
          rows={2}
        />
        <div className="composer-footer">
          <select value={mode} onChange={(event) => setMode(event.target.value as "plan" | "yolo")} aria-label="运行模式" disabled={running}>
            <option value="yolo">工作模式</option>
            <option value="plan">规划模式</option>
          </select>
          <span>Enter 发送，Shift+Enter 换行</span>
          {running ? (
            <button type="button" className="send-button stop" onClick={() => void run.stop()} aria-label="停止运行"><Square size={15} fill="currentColor" /></button>
          ) : (
            <button type="submit" className="send-button" disabled={!input.trim() || !activeSession} aria-label="发送消息"><ArrowUp size={17} /></button>
          )}
        </div>
      </form>
    </div>
  );
}

function HistoryMessage({ message }: { message: HistoryEntry }) {
  if (message.role === "user") return <article className="message user-message"><div className="message-content">{message.content}</div></article>;
  return (
    <article className="message assistant-message">
      <ReasoningBlock source={message.reasoning ?? ""} />
      <MarkdownRenderer source={message.content} />
    </article>
  );
}
