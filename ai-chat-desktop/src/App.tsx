import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import SmartContentRenderer from "./components/SmartContentRenderer";
import Sidebar from "./components/Sidebar";
import "./App.css"; // Keep this import for now, even if empty
import McpToolsMenu from "./components/McpToolsMenu";

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

function App() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  const safeInvoke = async (cmd: string, args?: any) => {
    if (typeof window.__TAURI_IPC__ !== "undefined") {
      return await invoke(cmd, args);
    } else {
      console.warn(`Tauri IPC not available. Command "${cmd}" not executed.`);
      throw new Error(`Tauri IPC not available for command: ${cmd}`);
    }
  };

  // 加载所有会话列表
  const loadSessions = async () => {
    try {
      const list = await safeInvoke("get_all_sessions");
      console.log("loadSessions: Received list from backend:", list); // Debugging line
      setSessions(list as ChatSession[]);
      console.log("loadSessions: Sessions after setSessions:", list); // Debugging line
    } catch (error) {
      console.error("loadSessions: Error loading sessions:", error); // Catch and log errors
      setSessions([]); // Ensure sessions is empty on error
    }
  };

  // 加载当前会话内容
  const loadCurrentSession = async () => {
    try {
      const session = await safeInvoke("get_current_session");
      const s = session as any;
      setCurrentSessionId(s.id);
      setMessages(s.messages || []);
    } catch {
      setMessages([]);
      setCurrentSessionId(null);
    }
  };

  useEffect(() => {
    loadSessions();
    loadCurrentSession();
  }, []);

  // 新建会话
  const handleNewChat = async () => {
    try {
      await safeInvoke("finalize_and_new_chat");
      await loadSessions();
      await loadCurrentSession();
    } catch (error) {
      console.error("Failed to create new chat:", error);
      alert(`Error creating new chat: ${error}`);
    }
  };

  // 切换会话
  const handleSelectSession = async (id: string) => {
    try {
      const new_session = await safeInvoke("select_session", { idToSelect: id });
      await loadSessions(); // 重新加载会话列表以更新标题
      // 使用返回的数据更新当前会话视图
      const s = new_session as ChatSession;
      setCurrentSessionId(s.id);
      setMessages(s.messages || []);
    } catch (error) {
      console.error("Failed to switch session:", error);
      alert(`Error switching session: ${error}`);
    }
  };

  // 重命名会话
  const handleRenameChat = async (id: string, newTitle: string) => {
    try {
      console.log("handleRenameChat: Attempting to rename session:", id, "to", newTitle); // Debugging line
      await safeInvoke("rename_session", { id, newTitle });
      console.log("handleRenameChat: Session renamed successfully."); // Debugging line
      await loadSessions();
      if (id === currentSessionId) {
        // If the current session was renamed, update its title in the view
        setSessions((prev) =>
          prev.map((s) => (s.id === id ? { ...s, title: newTitle } : s))
        );
      }
    } catch (error) {
      console.error("handleRenameChat: Failed to rename session:", error); // Catch and log errors
      alert(`Error renaming session: ${error}`);
    }
  };

  // 删除会话
  const handleDeleteChat = async (id: string) => {
    try {
      console.log("handleDeleteChat: Attempting to delete session:", id); // Debugging line
      await safeInvoke("delete_session", { id });
      console.log("handleDeleteChat: Session deleted successfully."); // Debugging line
      await loadSessions();
      if (id === currentSessionId) {
        // If the current session was deleted, reset current session
        setCurrentSessionId(null);
        setMessages([]);
      }
    } catch (error) {
      console.error("handleDeleteChat: Failed to delete session:", error); // Catch and log errors
      alert(`Error deleting session: ${error}`);
    }
  };

  // 发送消息
  const handleSendMessage = async () => {
    const trimmedValue = inputValue.trim();
    if (!trimmedValue || isLoading) return;
    setInputValue("");
    setIsLoading(true);
    setMessages((prev) => [
      ...prev,
      { role: "user", content: trimmedValue, timestamp: Date.now() },
    ]);
    try {
      const reply = await safeInvoke("send_message_to_openai", { message: trimmedValue });
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: reply as string, timestamp: Date.now() },
      ]);
      await loadSessions();
    } catch (error) {
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: `❌ Error: ${error}`, timestamp: Date.now() },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  // 左下角Config按钮回调
  const handleConfigOpenAI = () => {
    safeInvoke("open_config_file").catch(console.error);
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
        <div className="p-4 bg-gray-900 flex items-center space-x-3 border-t border-gray-700 backdrop-blur-md">
          <McpToolsMenu />
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
  );
}

export default App;