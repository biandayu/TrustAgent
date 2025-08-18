import React, { useState } from "react";

interface SessionItem {
  id: string;
  title: string;
}

interface SidebarProps {
  sessions: SessionItem[];
  currentSessionId: string | null;
  onSelect: (id: string) => void;
  onNewChat: () => void;
  collapsed: boolean;
  onToggle: () => void;
  onConfigOpenAI: () => void;
  onRenameChat: (id: string, newTitle: string) => void;
  onDeleteChat: (id: string) => void;
}

const Sidebar: React.FC<SidebarProps> = ({
  sessions,
  currentSessionId,
  onSelect,
  onNewChat,
  collapsed,
  onToggle,
  onConfigOpenAI,
  onRenameChat,
  onDeleteChat,
}) => {
  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [menuOpenSessionId, setMenuOpenSessionId] = useState<string | null>(null);
  const [newTitle, setNewTitle] = useState<string>("");
  return (
  <div className={`relative bg-gray-900 text-white flex flex-col border-r border-gray-700 transition-all duration-200 ease-in-out ${collapsed ? "w-12 min-w-[48px] max-w-[48px]" : "w-[220px] min-w-[180px] max-w-[320px]"}`}> 
      <div className="flex items-center p-3 border-b border-gray-700 bg-gray-900">
        <button className="bg-transparent border-none text-white text-lg cursor-pointer mr-2 w-7 h-7 rounded-md hover:bg-gray-700 transition-colors" onClick={onToggle} title={collapsed ? "Expand" : "Collapse"}>
          {collapsed ? ">" : "<"}
        </button>
        {!collapsed && <span className="font-bold text-base tracking-wide">Chats</span>}
      </div>
      {!collapsed && (
        <div className="flex-1 flex flex-col px-3 overflow-y-auto">
          <button className="mx-0 mt-4 mb-2 py-2 w-full bg-gradient-to-r from-blue-600 to-purple-700 text-white rounded-md font-bold text-sm cursor-pointer transition-all duration-200 ease-in-out hover:from-blue-700 hover:to-purple-800" onClick={onNewChat}>+ New Chat</button>
          <div className="font-bold text-sm text-gray-400 px-0 pt-3 mt-3 border-b border-gray-700 pb-2 mb-2 uppercase">Chat History</div>
          <div className="flex-1 overflow-y-auto px-0 pb-2">
            {sessions.map((s) => (
              <div
                key={s.id}
                className={`group relative flex justify-between items-center py-2 px-3 rounded-md mb-1 cursor-pointer text-sm text-gray-300 transition-colors duration-200 ease-in-out hover:bg-gray-800 hover:text-white ${s.id === currentSessionId ? "bg-blue-700 text-white font-bold" : ""}`} // Missing backtick was here
                onClick={editingSessionId === s.id ? undefined : () => onSelect(s.id)}
                title={s.title}
              >
                {editingSessionId === s.id ? (
                  <input
                    type="text"
                    value={newTitle}
                    onChange={(e) => setNewTitle(e.target.value)}
                    onBlur={() => {
                      console.log("onBlur fired for session:", s.id); // Debugging line
                      if (newTitle.trim() !== "" && newTitle !== s.title) {
                        onRenameChat(s.id, newTitle);
                      }
                      setEditingSessionId(null);
                    }}
                    onKeyDown={(e) => {
                      console.log("onKeyDown fired for session:", s.id, "key:", e.key); // Debugging line
                      if (e.key === "Enter") {
                        e.currentTarget.blur();
                      }
                    }}
                    autoFocus
                    className="flex-1 bg-gray-800 border border-gray-700 rounded-md text-white px-2 py-1 text-sm mr-2"
                  />
                ) : (
                  <span className="flex-1 whitespace-nowrap overflow-hidden text-ellipsis">{s.title}</span>
                )}
                <div className="flex items-center space-x-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                  <button
                    className="bg-transparent border-none text-gray-400 text-lg cursor-pointer px-1 py-0.5 rounded-md hover:bg-gray-700 transition-colors"
                    onClick={(e) => {
                      e.stopPropagation(); // Prevent selecting the chat
                      setMenuOpenSessionId(menuOpenSessionId === s.id ? null : s.id);
                    }}
                  >
                    ...
                  </button>
                  {menuOpenSessionId === s.id && (
                    <div className="absolute top-full right-0 bg-gray-800 rounded-md shadow-lg z-10 min-w-[120px] overflow-hidden">
                      <div
                        className="px-3 py-2 text-gray-300 cursor-pointer hover:bg-gray-700 transition-colors"
                        onClick={(e) => {
                          e.stopPropagation();
                          setNewTitle(s.title);
                          setEditingSessionId(s.id);
                          setMenuOpenSessionId(null);
                        }}
                      >
                        Rename
                      </div>
                      <div
                        className="px-3 py-2 text-gray-300 cursor-pointer hover:bg-gray-700 transition-colors"
                        onClick={(e) => {
                          e.stopPropagation();
                          onDeleteChat(s.id);
                          setMenuOpenSessionId(null);
                        }}
                      >
                        Delete
                      </div>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
      {/* 左下角Config按钮 */}
      {!collapsed && (
        <div className="absolute bottom-0 left-0 w-full p-4 bg-gray-900 border-t border-gray-700 flex justify-start">
          <button className="w-full py-3 bg-gradient-to-r from-blue-600 to-purple-700 text-white rounded-md font-bold text-sm cursor-pointer transition-all duration-200 ease-in-out hover:from-blue-700 hover:to-purple-800 flex items-center justify-center space-x-2" onClick={onConfigOpenAI}>
            <svg className="w-5 h-5 text-white" width="20" height="20" viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg">
              <path d="M10 13.5A3.5 3.5 0 1 0 10 6.5a3.5 3.5 0 0 0 0 7zm7.43-2.06l-1.13-.18a6.97 6.97 0 0 0-.36-1.02l.7-.92a.75.75 0 0 0-.07-.98l-1.06-1.06a.75.75 0 0 0-.98-.07l-.92.7c-.33-.19-.68-.34-1.02-.46l-.18-1.13A.75.75 0 0 0 12.25 4h-1.5a.75.75 0 0 0-.74.62l-.18 1.13c-.35.12-.69.27-1.02.46l-.92-.7a.75.75 0 0 0-.98.07L4.85 6.88a.75.75 0 0 0-.07.98l.7.92c-.19.33-.34.68-.46 1.02l-1.13.18A.75.75 0 0 0 2 9.75v1.5c0 .37.27.68.62.74l1.13.18c.12.35.27.69.46 1.02l-.7.92a.75.75 0 0 0 .07.98l1.06 1.06c.28.28.72.3.98.07l.92-.7c.33.19.68.34 1.02.46l.18 1.13c.06.35.37.62.74.62h1.5c.37 0 .68-.27.74-.62l.18-1.13c.35-.12.69-.27 1.02-.46l.92.7c.26.23.7.21.98-.07l1.06-1.06a.75.75 0 0 0 .07-.98l-.7-.92c.19-.33.34-.68.46-1.02l1.13-.18c.35-.06.62-.37.62-.74v-1.5a.75.75 0 0 0-.62-.74zM10 15a5 5 0 1 1 0-10 5 5 0 0 1 0 10z" fill="currentColor"/>
            </svg>
            <span>Config</span>
          </button>
        </div>
      )}
    </div>
  );
};

export default Sidebar;