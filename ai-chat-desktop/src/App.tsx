import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import SmartContentRenderer from "./components/SmartContentRenderer";
import Sidebar from "./components/Sidebar";
import "./App.css"; // Keep this import for now, even if empty
import McpToolsMenu from "./components/McpToolsMenu";

// --- TypeScript Interfaces ---

interface ChatMessage {
  role: string;
  content: string;
  timestamp: number;
}

interface ChatSession {
  id: string;
  title: string;
  messages: ChatMessage[];
  created_at: number;
  updated_at: number;
}

interface AgentStatus {
  type: "thinking" | "using_tool";
  data?: {
    tool_name: string;
  };
}

interface AgentEvent {
  status: AgentStatus | null;
}

// --- Main App Component ---

function App() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [agentStatus, setAgentStatus] = useState<AgentStatus | null>(null);
  const [activeTools, setActiveTools] = useState<string[]>([]);

  const safeInvoke = async (cmd: string, args?: any) => {
    if (typeof window.__TAURI_IPC__ !== "undefined") {
      return await invoke(cmd, args);
    } else {
      console.warn(`Tauri IPC not available. Command "${cmd}" not executed.`);
      throw new Error(`Tauri IPC not available for command: ${cmd}`);
    }
  };

  // --- Session Management ---

  const loadSessions = async () => {
    try {
      const list = await safeInvoke("get_all_sessions");
      setSessions(list as ChatSession[]);
    } catch (error) {
      console.error("Error loading sessions:", error);
      setSessions([]);
    }
  };

  useEffect(() => {
    const initializeApp = async () => {
      try {
        const loadedSessions = (await safeInvoke("get_all_sessions")) as ChatSession[];
        setSessions(loadedSessions);

        if (loadedSessions.length > 0) {
          // FIX: Instead of just setting state, call the handler that syncs with the backend.
          await handleSelectSession(loadedSessions[0].id);
        } else {
          await handleNewChat();
        }
      } catch (error) {
        console.error("Error initializing app:", error);
        alert(`Failed to initialize the application: ${error}`);
      }
    };

    initializeApp();

    const unlisten = listen<AgentEvent>("agent_event", (event) => {
      console.log("Received agent_event:", event.payload);
      setAgentStatus(event.payload.status);
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const handleNewChat = async () => {
    try {
      // 1. Backend creates a new session and returns it.
      const newSession = (await safeInvoke("finalize_and_new_chat")) as ChatSession;

      // 2. Update frontend state with the new session.
      setCurrentSessionId(newSession.id);
      setMessages(newSession.messages || []);

      // 3. Add the new session to the list of all sessions.
      setSessions((prev) => [newSession, ...prev]);

    } catch (error) {
      console.error("Failed to create new chat:", error);
      alert(`Error creating new chat: ${error}`);
    }
  };

  const handleSelectSession = async (id: string) => {
    try {
      const new_session = await safeInvoke("select_session", { idToSelect: id });
      await loadSessions();
      const s = new_session as ChatSession;
      setCurrentSessionId(s.id);
      setMessages(s.messages || []);
    } catch (error) {
      console.error("Failed to switch session:", error);
      alert(`Error switching session: ${error}`);
    }
  };

  const handleRenameChat = async (id: string, newTitle: string) => {
    try {
      await safeInvoke("rename_session", { id, newTitle });
      await loadSessions();
      if (id === currentSessionId) {
        setSessions((prev) =>
          prev.map((s) => (s.id === id ? { ...s, title: newTitle } : s))
        );
      }
    } catch (error) {
      console.error("Failed to rename session:", error);
      alert(`Error renaming session: ${error}`);
    }
  };

  const handleDeleteChat = async (id: string) => {
    try {
      await safeInvoke("delete_session", { id });
      await loadSessions();
      if (id === currentSessionId) {
        setCurrentSessionId(null);
        setMessages([]);
      }
    } catch (error) {
      console.error("Failed to delete session:", error);
      alert(`Error deleting session: ${error}`);
    }
  };

  // --- Tool Management ---
  const handleToggleTool = (toolName: string) => {
    setActiveTools((prev) =>
      prev.includes(toolName)
        ? prev.filter((t) => t !== toolName)
        : [...prev, toolName]
    );
  };

  // --- Message Handling ---

  const handleSendMessage = async () => {
    const trimmedValue = inputValue.trim();
    if (!trimmedValue || isLoading) return;

    setInputValue("");
    setIsLoading(true);
    setAgentStatus(null); // Reset status on new message

    setMessages((prev) => [
      ...prev,
      { role: "user", content: trimmedValue, timestamp: Date.now() },
    ]);

    try {
      const reply = await safeInvoke("run_agent_task", { 
        message: trimmedValue,
        activeTools: activeTools,
      });
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: reply as string, timestamp: Date.now() },
      ]);
      await loadSessions(); // To update title if it was a new chat
    } catch (error) {
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: `❌ Error: ${error}`, timestamp: Date.now() },
      ]);
    } finally {
      setIsLoading(false);
      setAgentStatus(null); // Clear status when done
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  const handleConfigOpenAI = () => {
    safeInvoke("open_config_file").catch(console.error);
  };

  // --- Render Logic ---

  const renderAgentStatus = () => {
    if (!isLoading || !agentStatus) return null;

    let statusText = "";
    if (agentStatus.type === "thinking") {
      statusText = "Thinking...";
    } else if (agentStatus.type === "using_tool") {
      statusText = `Using tool: ${agentStatus.data?.tool_name}...`;
    }

    return (
      <div className="text-center text-xs text-gray-400 pb-2 animate-pulse">
        {statusText}
      </div>
    );
  };

  return (
    <div className="flex h-screen bg-gray-800 text-white">
      <Sidebar
        sessions={sessions}
        currentSessionId={currentSessionId}
        onConfigOpenAI={handleConfigOpenAI}
        onSelect={handleSelectSession}
        onNewChat={handleNewChat}
        collapsed={sidebarCollapsed}
        onToggle={() => setSidebarCollapsed((c) => !c)}
        onRenameChat={handleRenameChat}
        onDeleteChat={handleDeleteChat}
      />
      <div className="flex flex-col flex-1">
        <div className="flex-1 overflow-y-auto p-4">
          {messages.map((message, idx) => (
            <div
              key={idx}
              className={`p-3 rounded-lg mb-2 max-w-[85%] word-wrap break-words relative animate-messageSlideIn ${
                message.role === "user"
                  ? "bg-blue-600 ml-auto text-right shadow-lg shadow-blue-500/30"
                  : "bg-gray-700 mr-auto border border-gray-600 backdrop-blur-md shadow-lg shadow-black/20"
              }`}
            >
              {message.role === "assistant" || message.role === "bot" ? (
                <SmartContentRenderer content={message.content} />
              ) : (
                <div className="text-white font-medium leading-relaxed">{message.content}</div>
              )}
            </div>
          ))}
        </div>
        <div className="p-4 bg-gray-900 border-t border-gray-700 backdrop-blur-md">
          {renderAgentStatus()}
          <div className="flex items-center space-x-3">
            <McpToolsMenu 
              activeTools={activeTools}
              onToggleTool={handleToggleTool}
            />
            <input
              type="text"
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              onKeyPress={handleKeyPress}
              placeholder="💬 Type your message... Try requesting JSON, Markdown, HTML, or XML content"
              disabled={isLoading}
              className="flex-1 bg-gray-800 border border-gray-700 rounded-lg p-3 focus:outline-none focus:ring-2 focus:ring-blue-500 text-white placeholder-gray-400 transition-all duration-300 ease-in-out disabled:opacity-60 disabled:cursor-not-allowed"
            />
            <button
              onClick={handleSendMessage}
              disabled={isLoading || !inputValue.trim()}
              className="px-5 py-3 rounded-lg bg-gradient-to-r from-blue-600 to-purple-700 text-white font-semibold cursor-pointer transition-all duration-300 ease-in-out shadow-lg shadow-blue-500/30 hover:from-blue-700 hover:to-purple-800 hover:translate-y-[-2px] active:translate-y-0 disabled:opacity-60 disabled:cursor-not-allowed disabled:shadow-none disabled:hover:translate-y-0"
            >
              {isLoading ? "Sending..." : "Send"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;