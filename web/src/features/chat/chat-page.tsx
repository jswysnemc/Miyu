import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowDown } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../../api/client";
import { useChatAgentContext } from "../agents/chat-agent-context";
import { ChatComposer } from "./chat-composer";
import { HistoryTurn, LiveRunMessage } from "./chat-message";
import { MessageOverviewRail } from "./message-overview-rail";
import { createLiveOverviewItem, createTimelineOverviewItems } from "./message-overview-utils";
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
  const workspaces = useQuery({ queryKey: ["workspaces"], queryFn: api.workspaces.list });
  const activeSession = sessions.data?.find((session) => session.active);
  const timeline = useQuery({
    queryKey: ["timeline", activeSession?.id],
    queryFn: () => api.sessions.timeline(activeSession!.id),
    enabled: Boolean(activeSession)
  });
  const onSettled = useCallback(() => {
    void Promise.all([
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
      queryClient.invalidateQueries({ queryKey: ["todos"] })
    ]);
  }, [queryClient]);
  const onWorkspaceChanged = useCallback(() => {
    void Promise.all([
      queryClient.invalidateQueries({ queryKey: ["file-tree"] }),
      queryClient.invalidateQueries({ queryKey: ["file"] }),
      queryClient.invalidateQueries({ queryKey: ["workspace-diff"] })
    ]);
  }, [queryClient]);
  const run = useRunStream(workspaces.data?.active_id, activeSession?.id, onSettled, onWorkspaceChanged);
  const chatModel = useChatModel();
  const chatAgent = useChatAgentContext();
  const thinking = useThinkingLevel();
  const [input, setInput] = useState("");
  const [mode, setMode] = useState<"plan" | "yolo">("yolo");
  const composerAttachments = useComposerAttachments();
  const scrollRef = useRef<HTMLDivElement>(null);
  const scrollContentSignal = useMemo(() => [timeline.data, run.states], [timeline.data, run.states]);
  const { showJump, jumpToBottom, pauseFollowing } = useFollowOutputScroll(scrollRef, scrollContentSignal, activeSession?.id);

  useEffect(() => {
    run.reset();
    setInput("");
    composerAttachments.clearAttachments();
  }, [activeSession?.id]);

  /** 提交当前输入内容和模型选择。 */
  const submit = async () => {
    const value = input.trim();
    if ((!value && composerAttachments.attachments.length === 0) || !activeSession) return;
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

  const runningStates = run.states.filter((state) => !state.completed);
  const activeRun = runningStates.find((state) => state.status !== "queued") ?? runningStates[0];
  const running = runningStates.length > 0;
  const historyEntries = timeline.data?.map((turn) => turn.user.content) ?? [];
  const overviewItems = useMemo(
    () => [
      ...createTimelineOverviewItems(timeline.data ?? []),
      ...run.states.map(createLiveOverviewItem).filter((item) => item !== null)
    ],
    [timeline.data, run.states]
  );

  /** 用最后一轮的用户输入重新发起运行，实时轮存在时携带其图片附件。 */
  const retry = async () => {
    if (!activeSession || running) return;
    // 1. 优先取实时运行的输入，否则回退到最后一条历史轮次
    const latestRun = run.states.at(-1);
    const liveInput = latestRun?.userInput ?? "";
    const content = liveInput || timeline.data?.at(-1)?.user.content || "";
    const liveImages = latestRun?.imageUrls;
    if (!content.trim() && !(liveImages && liveImages.length > 0)) return;
    // 2. 复用当前模式、模型与思考等级重新提交
    await queryClient.invalidateQueries({ queryKey: ["timeline", activeSession.id] });
    await run.start(activeSession.id, content, mode, chatModel.selection ?? undefined, liveImages, thinking.thinkingLevel, chatAgent.selection?.id);
  };
  const lastTurnId = timeline.data?.at(-1)?.turn_id;
  const historyRetry = run.states.length === 0 && !running ? () => void retry() : undefined;
  const emptySession = !timeline.isLoading && timeline.data?.length === 0 && run.states.length === 0;
  return (
    <div className={emptySession ? "chat-page empty-session" : "chat-page"}>
      <header className="chat-header">
        <h1>{activeSession?.title ?? "选择会话"}</h1>
      </header>
      <div className="message-scroll-region">
        <div className="message-scroll" ref={scrollRef}>
          <div className="message-column">
            {timeline.isLoading && <div className="empty-chat">正在读取会话历史</div>}
            {timeline.data?.map((turn) => (
              <section className="conversation-turn" data-overview-id={`turn-${turn.turn_id}`} key={turn.turn_id}>
                <HistoryTurn turn={turn} onRetry={turn.turn_id === lastTurnId ? historyRetry : undefined} />
              </section>
            ))}
            {run.states.map((state) => (
              <section className="conversation-turn" data-overview-id={`live-${state.runId}`} key={state.runId}>
                <LiveRunMessage
                  state={state}
                  running={!state.completed}
                  onRetry={!running && state.completed ? () => void retry() : undefined}
                />
              </section>
            ))}
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
      {emptySession && (
        <div className="empty-session-greeting">
          <h2>开始新的对话</h2>
          <p>输入任务或问题，Enter 发送，Shift+Enter 换行</p>
        </div>
      )}
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
        runStatus={activeRun?.status ?? "idle"}
        sessionAvailable={Boolean(activeSession)}
        agentChoices={chatAgent.choices}
        agentSelection={chatAgent.selection}
        agentLoading={chatAgent.isLoading}
        sessionId={activeSession?.id}
        onChange={setInput}
        onModeChange={setMode}
        onThinkingLevelChange={thinking.setThinkingLevel}
        onAddImages={composerAttachments.addFiles}
        onRemoveAttachment={composerAttachments.removeAttachment}
        onModelSelect={chatModel.selectModel}
        onSubmit={() => void submit()}
        onStop={() => activeRun?.runId && void run.stop(activeRun.runId)}
        onAgentSelect={chatAgent.selectAgent}
      />
    </div>
  );
}
