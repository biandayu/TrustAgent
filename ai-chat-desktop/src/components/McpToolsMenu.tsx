import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import '../App.css';

// --- Interfaces ---

interface McpServerInfo {
  name: string;
  status: 'running' | 'stopped';
}

// The backend returns a list of tool names (string[]), not complex objects.
// The McpTool interface was incorrect and has been removed.

interface Props {
  activeTools: string[];
  onToggleTool: (toolName: string) => void;
}

// --- Component ---

const McpToolsMenu: React.FC<Props> = ({ activeTools, onToggleTool }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [selectedServerName, setSelectedServerName] = useState<string | null>(null);
  const [servers, setServers] = useState<McpServerInfo[]>([]);
  // FIX: The state should hold a record of server names to their tool name strings.
  const [discoveredTools, setDiscoveredTools] = useState<Record<string, string[]>>({});
  const menuRef = useRef<HTMLDivElement>(null);

  // --- Data Fetching ---

  const loadServerStatus = async () => {
    try {
      const serverList = (await invoke("get_mcp_servers")) as McpServerInfo[];
      setServers(serverList);
      // If servers are already running on startup, fetch their tools automatically.
      serverList.forEach((server) => {
        if (server.status === "running") {
          fetchToolsForServer(server.name);
        }
      });
    } catch (error) {
      console.error("Failed to load MCP servers:", error);
    }
  };

  const fetchToolsForServer = async (serverName: string) => {
    try {
        const tools = await invoke('get_discovered_tools', { serverName });
        setDiscoveredTools((prev) => ({ ...prev, [serverName]: tools as string[] }));
    } catch (error) {
        console.error(`Failed to fetch tools for ${serverName}:`, error);
    }
  }

  // --- Lifecycle & Listeners ---

  useEffect(() => {
    loadServerStatus();

    const unlisten = listen('mcp_server_status_changed', (event) => {
      console.log('Received mcp_server_status_changed event:', event);
      loadServerStatus();
    });

    return () => {
      unlisten.then(f => f());
    };
  }, []);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setIsOpen(false);
        setSelectedServerName(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // --- Event Handlers ---

  const handleServerClick = (server: McpServerInfo) => {
    if (server.status !== 'running') return;
    // If tools for this server haven't been fetched yet, fetch them now.
    if (!discoveredTools[server.name]) {
        fetchToolsForServer(server.name);
    }
    setSelectedServerName(server.name);
  }

  // --- UI Rendering ---

  const renderServerList = () => (
    <div className="mcp-menu-content">
      {servers.map((server) => {
        const toolCount = discoveredTools[server.name]?.length ?? 0;
        return (
          <div key={server.name} className="mcp-menu-item server-item">
            <div className="server-info" onClick={() => handleServerClick(server)}>
                <span className="server-name">{server.name}</span>
                <span className={`tool-count-badge ${server.status === 'running' ? 'active' : 'inactive'}`}>{toolCount}</span>
            </div>
          </div>
        );
      })}
    </div>
  );

  const renderToolList = () => {
    if (!selectedServerName) return null;
    const tools = discoveredTools[selectedServerName] || [];

    return (
      <div className="mcp-menu-content">
        <div className="mcp-menu-header">
          <button onClick={() => setSelectedServerName(null)} className="back-button">←</button>
          <h4>{selectedServerName}</h4>
        </div>
        <hr className="mcp-separator" />
        {tools.length > 0 ? tools.map((toolName) => (
          <label key={toolName} className="mcp-menu-item tool-item" title={toolName}>
            <input 
              type="checkbox" 
              className="tool-checkbox"
              checked={true}
              onChange={() => onToggleTool(toolName)}
            />
            <span className="tool-name">{toolName}</span>
          </label>
        )) : <div className="no-tools-message">No tools found for this server.</div>}
      </div>
    );
  };

  return (
    <div className="mcp-tools-menu" ref={menuRef}>
      <button className="tools-button" onClick={() => setIsOpen(!isOpen)}>
        Tools
      </button>
      {isOpen && (
        <div className="mcp-menu-popup">
          {selectedServerName ? renderToolList() : renderServerList()}
        </div>
      )}
    </div>
  );
};

export default McpToolsMenu;