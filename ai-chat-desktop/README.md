# TrustAgent Desktop

TrustAgent Desktop is a Tauri-based desktop application that combines the power of large language models (LLMs) with the ability to interact with external tools and systems. It provides an intuitive chat interface where users can engage in conversations with an AI assistant, which can leverage various tools to perform tasks and retrieve information.

## Key Features

### 🤖 AI Chat with Tool Integration
- Engage in natural language conversations with an AI assistant powered by OpenAI GPT models.
- The AI can intelligently decide to use external tools to fulfill requests.
- Real-time status updates indicate when the AI is "thinking" or "using a tool".

### 🧰 Extensible Tool Ecosystem (MCP)
- Integrates with Multi-Client Protocol (MCP) servers to discover and utilize a wide range of tools.
- Currently supports tools like:
  - **Playwright**: For web automation, including searching and scraping.
  - **Postal**: A custom tool server (example implementation).
- Easily add new MCP servers and tools by configuring them in `settings.json`.

### 📦 Persistent Chat Sessions
- Create, manage, rename, and delete multiple independent chat sessions.
- Chat history is automatically saved locally.

### 🎨 Intelligent Content Rendering
The application can automatically detect and beautifully display content in various formats returned by the AI:

- **Markdown**: Headers, paragraphs, lists, code blocks (with syntax highlighting), tables, quotes, links, etc.
- **JSON**: Syntax highlighting and formatted indentation for easy reading.
- **XML**: Syntax highlighting for structured data.
- **HTML**: Live preview and source code viewing.
- **Plain Text**: Preserves original formatting.

## Getting Started

1. **Configuration**:
   - Locate the `settings.json` file (usually in the application's configuration directory or root).
   - Add your OpenAI API key:
     ```json
     {
       "openai": {
         "api_key": "YOUR_OPENAI_API_KEY_HERE",
         ...
       },
       ...
     }
     ```
   - Configure MCP servers as needed in the `mcpServers` section.

2. **Usage**:
   - Launch the TrustAgent Desktop application.
   - Start chatting with the AI in the main window.
   - Use the left sidebar to manage chat sessions.
   - Use the "Tools" menu (next to the chat input) to see available tools and enable/disable them.

## Development Setup

This project is built using Tauri, combining Rust (for the backend/desktop) and React/TypeScript (for the frontend).

### Prerequisites

- **Rust**: Install Rust via [rustup](https://rustup.rs/).
- **Node.js & npm**: Download and install from [nodejs.org](https://nodejs.org/).
- **System Dependencies**:
  - **Windows**: Windows 10 or later. Visual Studio C++ Build Tools.
  - **macOS**: macOS 10.15 or later. Xcode Command Line Tools (`xcode-select --install`).
  - **Linux**: `webkit2gtk` and other development libraries (see [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites/)).

### Running in Development Mode

1. Clone the repository.
2. Navigate to the `ai-chat-desktop` directory:
   ```bash
   cd ai-chat-desktop
   ```
3. Install frontend dependencies:
   ```bash
   npm install
   ```
4. Start the development server:
   ```bash
   npm run tauri dev
   ```

### Building for Release

To create a distributable application for your current operating system:

```bash
npm run tauri build
```

The built artifacts will be located in `src-tauri/target/release/bundle/`.

## Tech Stack

- **Frontend**: React + TypeScript
- **Styling**: Tailwind CSS
- **Desktop Framework**: Tauri (Rust)
- **AI Interaction**: `async-openai` (Rust)
- **Tool Communication**: `rmcp` (Rust Multi-Client Protocol library)
- **Content Rendering**:
  - `react-markdown`
  - `react-syntax-highlighter`
  - `rehype-highlight`
  - `remark-gfm`