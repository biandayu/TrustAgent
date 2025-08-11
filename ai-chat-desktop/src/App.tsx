import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import SmartContentRenderer from "./components/SmartContentRenderer";
import "./App.css";

function App() {
  const [messages, setMessages] = useState([
    { 
      id: 1, 
      text: "🎉 Welcome to AI Chat Desktop!\n\n✨ **New Features:**\n- 🌙 Dark theme interface\n- 📱 Responsive design, adapts to window size\n- 🎨 Smart content rendering (Markdown, JSON, XML, HTML)\n- 💫 Smooth animations\n\n💡 **Usage Tips:**\n- Resize the window, chat area will adapt automatically\n- Try requesting different content formats to experience smart rendering\n- Input box supports Enter key for quick sending", 
      sender: "bot" 
    },
  ]);
  const [inputValue, setInputValue] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    const loadKey = async () => {
      const key = await invoke("load_api_key");
      if (key) {
        setApiKey(key as string);
      }
    };
    loadKey();
  }, []);

  const handleSendMessage = async () => {
    const trimmedValue = inputValue.trim();
    if (!trimmedValue || isLoading) return;

    // Clear input immediately
    setInputValue("");
    setIsLoading(true);

    const userMessage = { id: Date.now(), text: trimmedValue, sender: "user" };
    setMessages(prevMessages => [...prevMessages, userMessage]);

    try {
      const reply = await invoke("send_message_to_openai", { message: trimmedValue });
      const botMessage = { id: Date.now() + 1, text: reply as string, sender: "bot" };
      setMessages(prevMessages => [...prevMessages, botMessage]);
    } catch (error) {
      const errorMessage = { 
        id: Date.now() + 1, 
        text: `❌ Error: ${error}`, 
        sender: "bot" 
      };
      setMessages(prevMessages => [...prevMessages, errorMessage]);
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
        {messages.map((message) => (
          <div key={message.id} className={`message ${message.sender}`}>
            {message.sender === "bot" ? (
              <SmartContentRenderer content={message.text} />
            ) : (
              <div className="user-message">{message.text}</div>
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
  );
}

export default App;