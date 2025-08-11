import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "../App.css";

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
}

const Sidebar: React.FC<SidebarProps> = ({
  sessions,
  currentSessionId,
  onSelect,
  onNewChat,
  collapsed,
  onToggle,
}) => {
  return (
    <div className={`sidebar${collapsed ? " collapsed" : ""}`}> 
      <div className="sidebar-header">
        <button className="sidebar-toggle" onClick={onToggle} title={collapsed ? "Expand" : "Collapse"}>
          {collapsed ? ">" : "<"}
        </button>
        {!collapsed && <span className="sidebar-title">Chats</span>}
      </div>
      {!collapsed && (
        <>
          <button className="new-chat-btn" onClick={onNewChat}>+ New Chat</button>
          <div className="session-list">
            {sessions.map((s) => (
              <div
                key={s.id}
                className={`session-item${s.id === currentSessionId ? " active" : ""}`}
                onClick={() => onSelect(s.id)}
                title={s.title}
              >
                <span className="session-title">{s.title}</span>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
};

export default Sidebar;
