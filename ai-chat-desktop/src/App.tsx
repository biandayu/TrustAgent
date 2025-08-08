import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

function App() {
  const [messages, setMessages] = useState([
    { id: 1, text: "Hello!", sender: "bot" },
  ]);
  const [inputValue, setInputValue] = useState("");
  const [apiKey, setApiKey] = useState("");

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
    if (inputValue.trim()) {
      const userMessage = { id: Date.now(), text: inputValue, sender: "user" };
      setMessages([...messages, userMessage]);
      try {
        const reply = await invoke("send_message_to_openai", { message: inputValue });
        const botMessage = { id: Date.now() + 1, text: reply as string, sender: "bot" };
        setMessages(prevMessages => [...prevMessages, botMessage]);
      } catch (error) {
        const errorMessage = { id: Date.now() + 1, text: `Error: ${error}`, sender: "bot" };
        setMessages(prevMessages => [...prevMessages, errorMessage]);
      }
      setInputValue("");
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
          placeholder="Enter your OpenAI API Key"
          value={apiKey}
          onChange={(e) => handleApiKeyChange(e.target.value)}
        />
      </div>
      <div className="message-list">
        {messages.map((message) => (
          <div key={message.id} className={`message ${message.sender}`}>
            {message.text}
          </div>
        ))}
      </div>
      <div className="message-input">
        <input
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyPress={(e) => e.key === "Enter" && handleSendMessage()}
        />
        <button onClick={handleSendMessage}>Send</button>
      </div>
    </div>
  );
}

export default App;