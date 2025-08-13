import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import '../App.css';

// --- Interfaces matching Rust structs ---
interface McpServerInfo {
  name: string;
  status: 'running' | 'stopped';
}

interface McpTool {
  name: string;
  description: string;
  // Assuming the schema has these fields, adjust if necessary
  input_schema: object;
  output_schema: object;
}

const McpToolsMenu: React.FC = () => {
  const [isOpen, setIsOpen] = useState(false);
  const [selectedServerName, setSelectedServerName] = useState<string | null>(null);
  const [servers, setServers] = useState<McpServerInfo[]>([]);
  const [discoveredTools, setDiscoveredTools] = useState<Record<string, McpTool[]>>({});
  const [isLoading, setIsLoading] = useState<Record<string, boolean>>({});

  const menuRef = useRef<HTMLDivElement>(null);

  // --- Data Fetching from Backend ---
  const loadServerStatus = async () => {
    try {
      const serverList = await invoke('get_mcp_servers');
      setServers(serverList as McpServerInfo[]);
    } catch (error) {
      console.error("Failed to load MCP servers:", error);
    }
  };

  useEffect(() => {
    loadServerStatus();
  }, []);

  // --- Event Handlers ---
  const handleStartServer = async (serverName: string) => {
    setIsLoading((prev) => ({ ...prev, [serverName]: true }));
    try {
      await invoke('start_mcp_server', { serverName });
      const tools = await invoke('get_discovered_tools', { serverName });
      setDiscoveredTools((prev) => ({ ...prev, [serverName]: tools as McpTool[] }));
      setServers((prev) =>
        prev.map((s) => (s.name === serverName ? { ...s, status: 'running' } : s))
      );
    } catch (error) {
      console.error(`Failed to start server ${serverName}:`, error);
      alert(`Error starting server ${serverName}: ${error}`);
    } finally {
      setIsLoading((prev) => ({ ...prev, [serverName]: false }));
    }
  };

  const handleStopServer = async (serverName: string) => {
    setIsLoading((prev) => ({ ...prev, [serverName]: true }));
    try {
      await invoke('stop_mcp_server', { serverName });
      setServers((prev) =>
        prev.map((s) => (s.name === serverName ? { ...s, status: 'stopped' } : s))
      );
      setDiscoveredTools((prev) => {
        const newState = { ...prev };
        delete newState[serverName];
        return newState;
      });
      setSelectedServerName(null); // Go back to server list if this one was selected
    } catch (error) {
      console.error(`Failed to stop server ${serverName}:`, error);
      alert(`Error stopping server ${serverName}: ${error}`);
    } finally {
      setIsLoading((prev) => ({ ...prev, [serverName]: false }));
    }
  };

  // --- UI Rendering ---
  const renderServerList = () => (
    <div className="mcp-menu-content">
      {servers.map((server) => {
        const tools = discoveredTools[server.name] || [];
        const serverIsLoading = isLoading[server.name];
        return (
          <div key={server.name} className="mcp-menu-item server-item">
            <div className="server-info" onClick={() => server.status === 'running' && setSelectedServerName(server.name)}>
                <span className={`status-indicator ${server.status}`}></span>
                <span className="server-name">{server.name}</span>
                {server.status === 'running' && <span className="tool-count-badge">{tools.length}</span>}
            </div>
            <button
              className={`server-action-btn ${server.status}`}
              onClick={() => server.status === 'running' ? handleStopServer(server.name) : handleStartServer(server.name)}
              disabled={serverIsLoading}
            >
              {serverIsLoading ? '...' : (server.status === 'running' ? 'Stop' : 'Start')}
            </button>
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
        {tools.length > 0 ? tools.map((tool) => (
          <div key={tool.name} className="mcp-menu-item tool-item">
            <span className="tool-name" title={tool.description}>{tool.name}</span>
            {/* Placeholder for toggle switch */}
          </div>
        )) : <div className="no-tools-message">No tools found for this server.</div>}
      </div>
    );
  };

  // --- Component Lifecycle ---
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