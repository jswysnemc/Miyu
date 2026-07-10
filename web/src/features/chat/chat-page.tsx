import { useQuery, useQueryClient } from "@tanstack/react-query";
import { WandSparkles } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../../api/client";
import { ChatComposer } from "./chat-composer";
import { HistoryTurn, LiveRunMessage } from "./chat-message";
import { useComposerAttachments } from "./composer/use-composer-attachments";
import { useChatModel } from "./use-chat-model";
import { useRunStream } from "./use-run-stream";
import { useThinkingLevel } from "./use-thinking-level";
import { WorkStatus } from "./work-status";
import "./chat-page.css";

/** 渲染当前会话历史、实时运行事件和消息输入区。 */
export function ChatPage() {
  const queryClient = useQueryClient();
  const sessions = useQuery({ queryKey: ["sessions"], queryFn: api.sessions.list });
  const activeSession = sessions.data?.find((session) => session.active);
  const timeline = useQuery({
    queryKey: ["timeline", activeSession?.id],
    queryFn: () => api.sessions.timeline(activeSession!.id),
    enabled: Boolean(activeSession)
  });
  const onSettled = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["sessions"] });
  }, [queryClient]);
  const onWorkspaceChanged = useCallback(() => {
    void Promise.all([
      queryClient.invalidateQueries({ queryKey: ["file-tree"] }),
      queryClient.invalidateQueries({ queryKey: ["file"] }),
      queryClient.invalidateQueries({ queryKey: ["workspace-diff"] })
    ]);
  }, [queryClient]);
  const run = useRunStream(onSettled, onWorkspaceChanged);
  const chatModel = useChatModel();
  const thinking = useThinkingLevel();
  const [input, setInput] = useState("");
  const [mode, setMode] = useState<"plan" | "yolo">("yolo");
  const composerAttachments = useComposerAttachments();
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    run.reset();
    setInput("");
    composerAttachments.clearAttachments();
  }, [activeSession?.id]);

  useEffect(() => {
    const element = scrollRef.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [timeline.data, run.state.content, run.state.reasoning, run.state.tools.length]);

  /** 提交当前输入内容和模型选择。 */
  const submit = async () => {
    const value = input.trim();
    if ((!value && composerAttachments.attachments.length === 0) || !activeSession || (run.state.runId && !run.state.completed)) return;
    await queryClient.invalidateQueries({ queryKey: ["timeline", activeSession.id] });
    const originalInput = input;
    const currentAttachments = composerAttachments.attachments;
    setInput("");
    composerAttachments.clearAttachments();
    await run.start(
      activeSession.id,
      value,
      mode,
      chatModel.selection ?? undefined,
      currentAttachments.map((attachment) => attachment.dataUrl),
      thinking.thinkingLevel
    ).catch((error: unknown) => {
      setInput(originalInput);
      composerAttachments.restoreAttachments(currentAttachments);
      throw error;
    });
  };

  const running = Boolean(run.state.runId && !run.state.completed);
  const historyEntries = timeline.data?.map((turn) => turn.user.content) ?? [];
  return (
    <div className="chat-page">
      <header className="chat-header">
        <h1>{activeSession?.title ?? "选择会话"}</h1>
        <WorkStatus status={run.state.status} />
      </header>
      <div className="message-scroll" ref={scrollRef}>
        <div className="message-column">
          {timeline.isLoading && <div className="empty-chat">正在读取会话历史</div>}
          {timeline.data?.length === 0 && !run.state.runId && (
            <div className="empty-chat">
              <span className="empty-mark"><WandSparkles size={23} /></span>
              <h2>从工作区开始</h2>
              <p>描述需要完成的代码任务，Miyu 会在当前工作区中分析、调用工具并实时汇报进度。</p>
            </div>
          )}
          {timeline.data?.map((turn) => <HistoryTurn key={turn.turn_id} turn={turn} />)}
          {run.state.runId && <LiveRunMessage state={run.state} running={running} />}
          {timeline.error && <div className="run-error">{timeline.error.message}</div>}
          {chatModel.error && <div className="run-error">{chatModel.error.message}</div>}
        </div>
      </div>
      <ChatComposer
        value={input}
        mode={mode}
        attachments={composerAttachments.attachments}
        historyEntries={historyEntries}
        thinkingLevel={thinking.thinkingLevel}
        choices={chatModel.choices}
        selection={chatModel.selection}
        modelLoading={chatModel.isLoading}
        running={running}
        sessionAvailable={Boolean(activeSession)}
        onChange={setInput}
        onModeChange={setMode}
        onThinkingLevelChange={thinking.setThinkingLevel}
        onAddImages={composerAttachments.addFiles}
        onRemoveAttachment={composerAttachments.removeAttachment}
        onModelSelect={chatModel.selectModel}
        onSubmit={() => void submit()}
        onStop={() => void run.stop()}
      />
    </div>
  );
}
