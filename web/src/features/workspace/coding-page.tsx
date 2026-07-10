import { useState } from "react";
import { ChatPage } from "../chat/chat-page";
import { SessionSidebar } from "../sessions/session-sidebar";
import { WorkspacePane } from "./workspace-pane";
import "./coding-page.css";

export function CodingPage() {
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  return (
    <div className="coding-layout">
      <aside className="coding-sidebar">
        <SessionSidebar />
      </aside>
      <section className="coding-chat">
        <ChatPage />
      </section>
      <aside className="coding-workspace">
        <WorkspacePane selectedFile={selectedFile} onSelectFile={setSelectedFile} />
      </aside>
    </div>
  );
}
