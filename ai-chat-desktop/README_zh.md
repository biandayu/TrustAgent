# TrustAgent 桌面版

TrustAgent 桌面版是一个基于 Tauri 构建的桌面应用程序，它将大型语言模型 (LLM) 的强大功能与与外部工具和系统交互的能力相结合。它提供了一个直观的聊天界面，用户可以在其中与 AI 助手进行对话，该助手可以利用各种工具来执行任务和检索信息。

## 核心功能

### 🤖 集成工具的 AI 聊天
- 与由 OpenAI GPT 模型驱动的 AI 助手进行自然语言对话。
- AI 可以智能地决定使用外部工具来完成请求。
- 实时状态更新，指示 AI 正在“思考”或“使用工具”。

### 🧰 可扩展的工具生态系统 (MCP)
- 集成多客户端协议 (MCP) 服务器，以发现和使用各种工具。
- 目前支持的工具包括：
  - **Playwright**：用于网页自动化，包括搜索和抓取。
  - **Postal**：一个自定义工具服务器（示例实现）。
- 通过在 `settings.json` 中配置，可以轻松添加新的 MCP 服务器和工具。

### 📦 持久化聊天会话
- 创建、管理、重命名和删除多个独立的聊天会话。
- 聊天记录会自动保存在本地。

### 🎨 智能内容渲染
应用程序可以自动检测并以美观的方式显示 AI 返回的各种格式的内容：

- **Markdown**：标题、段落、列表、代码块（带语法高亮）、表格、引用、链接等。
- **JSON**：语法高亮和格式化缩进，便于阅读。
- **XML**：结构化数据的语法高亮。
- **HTML**：实时预览和源代码查看。
- **纯文本**：保留原始格式。

## 开始使用

1.  **配置**：
    - 找到 `settings.json` 文件（通常位于应用程序的配置目录或根目录下）。
    - 添加您的 OpenAI API 密钥：
      ```json
      {
        "openai": {
          "api_key": "YOUR_OPENAI_API_KEY_HERE",
          ...
        },
        ...
      }
      ```
    - 根据需要在 `mcpServers` 部分配置 MCP 服务器。
2.  **使用**：
    - 启动 TrustAgent 桌面版应用程序。
    - 在主窗口中开始与 AI 聊天。
    - 使用左侧边栏管理聊天会话。
    - 使用聊天输入框旁边的“工具”菜单查看可用工具并启用/禁用它们。

## 开发环境搭建

本项目使用 Tauri 构建，结合了 Rust（用于后端/桌面）和 React/TypeScript（用于前端）。

### 先决条件

-   **Rust**：通过 [rustup](https://rustup.rs/) 安装 Rust。
-   **Node.js & npm**：从 [nodejs.org](https://nodejs.org/) 下载并安装。
-   **系统依赖**：
    -   **Windows**：Windows 10 或更高版本。Visual Studio C++ 生成工具。
    -   **macOS**：macOS 10.15 或更高版本。Xcode 命令行工具 (`xcode-select --install`)。
    -   **Linux**：`webkit2gtk` 和其他开发库（请参见 [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites/)）。

### 以开发模式运行

1.  克隆代码仓库。
2.  导航到 `ai-chat-desktop` 目录：
    ```bash
    cd ai-chat-desktop
    ```
3.  安装前端依赖：
    ```bash
    npm install
    ```
4.  启动开发服务器：
    ```bash
    npm run tauri dev
    ```

### 构建发布版

要为当前操作系统创建一个可分发的应用程序，请运行：

```bash
npm run tauri build
```

构建产物将位于 `src-tauri/target/release/bundle/` 目录下。

## 技术栈

-   **前端**：React + TypeScript
-   **样式**：Tailwind CSS
-   **桌面框架**：Tauri (Rust)
-   **AI 交互**：`async-openai` (Rust)
-   **工具通信**：`rmcp` (Rust 多客户端协议库)
-   **内容渲染**：
    -   `react-markdown`
    -   `react-syntax-highlighter`
    -   `rehype-highlight`
    -   `remark-gfm`