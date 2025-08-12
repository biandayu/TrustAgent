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
  onConfigOpenAI: () => void;
}

const Sidebar: React.FC<SidebarProps> = ({
  sessions,
  currentSessionId,
  onSelect,
  onNewChat,
  collapsed,
  onToggle,
  onConfigOpenAI,
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
      {/* 左下角Config按钮 */}
      {!collapsed && (
        <div className="sidebar-footer">
          <button className="config-openai-btn" onClick={onConfigOpenAI}>
            <span style={{display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 8, color: '#FFFFFF'}}>
              <svg className="config-gear-icon" width="20" height="20" viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg" style={{marginRight: 6}}>
                                <path d="M10 13.5A3.5 3.5 0 1 0 10 6.5a3.5 3.5 0 0 0 0 7zm7.43-2.06l-1.13-.18a6.97 6.97 0 0 0-.36-1.02l.7-.92a.75.75 0 0 0-.07-.98l-1.06-1.06a.75.75 0 0 0-.98-.07l-.92.7c-.33-.19-.68-.34-1.02-.46l-.18-1.13A.75.75 0 0 0 12.25 4h-1.5a.75.75 0 0 0-.74.62l-.18 1.13c-.35.12-.69.27-1.02.46l-.92-.7a.75.75 0 0 0-.98.07L4.85 6.88a.75.75 0 0 0-.07.98l.7.92c-.19.33-.34.68-.46 1.02l-1.13.18A.75.75 0 0 0 2 9.75v1.5c0 .37.27.68.62.74l1.13.18c.12.35.27.69.46 1.02l-.7.92a.75.75 0 0 0 .07.98l1.06 1.06c.28.28.72.3.98.07l.92-.7c.33.19.68.34 1.02.46l.18 1.13c.06.35.37.62.74.62h1.5c.37 0 .68-.27.74-.62l.18-1.13c.35-.12.69-.27 1.02-.46l.92.7c.26.23.7.21.98-.07l1.06-1.06a.75.75 0 0 0 .07-.98l-.7-.92c.19-.33.34-.68.46-1.02l1.13-.18c.35-.06.62-.37.62-.74v-1.5a.75.75 0 0 0-.62-.74zM10 15a5 5 0 1 1 0-10 5 5 0 0 1 0 10z" fill="#FFFFFF"/>
              </svg>
              <span className="config-gear-label">Config</span>
            </span>
          </button>
        </div>
      )}
    </div>
  );
};

export default Sidebar;