# AI Chat Desktop

A Tauri-based desktop AI chat application with intelligent content rendering.

## Features

### 🤖 AI Chat
- Chat with OpenAI GPT models
- Support for multiple content format intelligent rendering

### 🎨 Intelligent Content Rendering
The application can automatically detect and beautifully display content in the following formats:

#### 📝 Markdown
- Headers, paragraphs, lists
- Code blocks (with syntax highlighting)
- Tables, quotes, links
- Bold, italic, and other formatting

#### 🔧 JSON
- Syntax highlighting
- Formatted indentation
- Easy-to-read structure

#### 🏷️ XML
- Syntax highlighting
- Structured display
- Tag recognition

#### 🌐 HTML
- Live preview
- Source code viewing
- Collapsible display

#### 📄 Plain Text
- Preserves original formatting
- Monospace font display

## Usage

1. Enter your OpenAI API key
2. Start chatting with AI
3. Try requesting specific content formats, for example:
   - "Generate user data in JSON format"
   - "Write a Markdown document"
   - "Create an HTML table"
   - "Generate XML configuration file"

## Tech Stack

- **Frontend**: React + TypeScript
- **Desktop Framework**: Tauri
- **UI Components**: Custom components
- **Content Rendering**: 
  - react-markdown
  - react-syntax-highlighter
  - rehype-highlight
  - remark-gfm

## Development

```bash
# Install dependencies
npm install

# Development mode
npm run tauri dev

# Build
npm run tauri build
```

## Examples

### JSON Response
```json
{
  "name": "John Doe",
  "age": 30,
  "email": "john@example.com"
}
```

### Markdown Response
# Header
**Bold text** and *italic text*

- List item 1
- List item 2

```javascript
console.log("Code block");
```

### HTML Response
<table>
  <tr><th>Name</th><th>Age</th></tr>
  <tr><td>John</td><td>25</td></tr>
</table>

The application will automatically detect these formats and display them in the best way!
