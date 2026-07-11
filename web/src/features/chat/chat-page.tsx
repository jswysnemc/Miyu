import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowDown, WandSparkles } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../../api/client";
import { useChatAgentContext } from "../agents/chat-agent-context";
import { ChatComposer } from "./chat-composer";
import { HistoryTurn, LiveRunMessage } from "./chat-message";
import { MessageOverviewRail } from "./message-overview-rail";
import { createTimelineOverviewItems } from "./message-overview-utils";
import { useComposerAttachments } from "./composer/use-composer-attachments";
import { useChatModel } from "./use-chat-model";
import { useRunStream } from "./use-run-stream";
import { useThinkingLevel } from "./use-thinking-level";
import { useFollowOutputScroll } from "./use-follow-output-scroll";
import "./chat-page.css";

/**
 * 渲染当前会话历史、实时运行事件和消息输入区。
 *
 * @returns 聊天页面
 */
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
  const chatAgent = useChatAgentContext();
  const thinking = useThinkingLevel();
  const [input, setInput] = useState("");
  const [mode, setMode] = useState<"plan" | "yolo">("yolo");
  const composerAttachments = useComposerAttachments();
  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollContentSignal = useMemo(() => [timeline.data, run.state], [timeline.data, run.state]);
  const { showJump, jumpToBottom, pauseFollowing } = useFollowOutputScroll(scrollRef, scrollContentSignal, activeSession?.id);

  useEffect(() => {
    run.reset();
    setInput("");
    composerAttachments.clearAttachments();
  }, [activeSession?.id]);

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
      thinking.thinkingLevel,
      chatAgent.selection?.id
    ).catch((error: unknown) => {
      setInput(originalInput);
      composerAttachments.restoreAttachments(currentAttachments);
      throw error;
    });
  };

  const running = Boolean(run.state.runId && !run.state.completed);
  const historyEntries = timeline.data?.map((turn) => turn.user.content) ?? [];
  const overviewItems = useMemo(
    () => createTimelineOverviewItems(timeline.data ?? [], run.state.runId ? run.state : undefined),
    [timeline.data, run.state]
  );

  /** 用最后一轮的用户输入重新发起运行，实时轮存在时携带其图片附件。 */
  const retry = async () => {
    if (!activeSession || running) return;
    // 1. 优先取实时运行的输入，否则回退到最后一条历史轮次
    const liveInput = run.state.runId ? run.state.userInput : "";
    const content = liveInput || timeline.data?.at(-1)?.user.content || "";
    const liveImages = run.state.runId ? run.state.imageUrls : undefined;
    if (!content.trim() && !(liveImages && liveImages.length > 0)) return;
    // 2. 复用当前模式、模型与思考等级重新提交
    await queryClient.invalidateQueries({ queryKey: ["timeline", activeSession.id] });
    await run.start(activeSession.id, content, mode, chatModel.selection ?? undefined, liveImages, thinking.thinkingLevel, chatAgent.selection?.id);
  };
  const lastTurnId = timeline.data?.at(-1)?.turn_id;
  const historyRetry = !run.state.runId && !running ? () => void retry() : undefined;
  return (
    <div className="chat-page">
      <header className="chat-header">
        <h1>{activeSession?.title ?? "选择会话"}</h1>
      </header>
      <div className="message-scroll-region">
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
            {timeline.data?.map((turn) => (
              <section className="conversation-turn" data-overview-id={`turn-${turn.turn_id}`} key={turn.turn_id}>
                <HistoryTurn turn={turn} onRetry={turn.turn_id === lastTurnId ? historyRetry : undefined} />
              </section>
            ))}
            {run.state.runId && (
              <section className="conversation-turn" data-overview-id={`live-${run.state.runId}`}>
                <LiveRunMessage state={run.state} running={running} onRetry={running ? undefined : () => void retry()} />
              </section>
            )}
            {timeline.error && <div className="run-error">{timeline.error.message}</div>}
            {chatModel.error && <div className="run-error">{chatModel.error.message}</div>}
          </div>
        </div>
        <MessageOverviewRail
          scrollContainerRef={scrollRef}
          items={overviewItems}
          onNavigate={pauseFollowing}
        />
        {showJump && (
          <button type="button" className="jump-to-bottom" onClick={jumpToBottom} aria-label="回到底部" title="回到底部">
            <ArrowDown size={16} />
          </button>
        )}
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
        runStatus={run.state.status}
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
