import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import SmartContentRenderer from "./components/SmartContentRenderer";
import Sidebar from "./components/Sidebar";
import "./App.css";

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
  const [apiKey, setApiKey] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(null);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  // 加载所有会话列表
  const loadSessions = async () => {
    const list = await invoke("get_all_sessions");
    setSessions(list as ChatSession[]);
  };

  // 加载当前会话内容
  const loadCurrentSession = async () => {
    try {
      const session = await invoke("get_current_session");
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
    // 加载API Key
    invoke("load_api_key").then((key) => {
      if (key) setApiKey(key as string);
    });
  }, []);

  // 新建会话
  const handleNewChat = async () => {
    try {
      await invoke("finalize_and_new_chat");
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
      const new_session = await invoke("select_session", { idToSelect: id });
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
      const reply = await invoke("send_message_to_openai", { message: trimmedValue });
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

  const handleApiKeyChange = async (newKey: string) => {
    setApiKey(newKey);
    await invoke("save_api_key", { apiKey: newKey });
  };

  return (
    <div className="main-layout">
      <Sidebar
        sessions={sessions}
        currentSessionId={currentSessionId}
        onSelect={handleSelectSession}
        onNewChat={handleNewChat}
        collapsed={sidebarCollapsed}
        onToggle={() => setSidebarCollapsed((c) => !c)}
      />
      <div className="chat-container">
        <div className="api-key-input">
          <input
            type="password"
            placeholder="🔑 Enter your OpenAI API Key"
            value={apiKey}
            onChange={(e) => handleApiKeyChange(e.target.value)}
          />
        </div>
        <div className="message-list">
          {messages.map((message, idx) => (
            <div key={idx} className={`message ${message.role === "user" ? "user" : "bot"}`}>
              {message.role === "assistant" || message.role === "bot" ? (
                <SmartContentRenderer content={message.content} />
              ) : (
                <div className="user-message">{message.content}</div>
              )}
            </div>
          ))}
        </div>
        <div className="message-input">
          <input
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder="💬 Type your message... Try requesting JSON, Markdown, HTML, or XML content"
            disabled={isLoading}
          />
          <button
            onClick={handleSendMessage}
            disabled={isLoading || !inputValue.trim()}
          >
            {isLoading ? "Sending..." : "Send"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;