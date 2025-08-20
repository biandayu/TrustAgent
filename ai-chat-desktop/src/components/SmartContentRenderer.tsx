import React from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import rehypeRaw from 'rehype-raw';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { tomorrow } from 'react-syntax-highlighter/dist/esm/styles/prism';
import 'prismjs/themes/prism-tomorrow.css'; // Directly import the CSS

// IMPORTANT: For syntax highlighting to work, ensure you have imported a Prism.js theme's CSS.
// For example, in your main CSS file (e.g., App.css or index.css), you might add:
// @import 'prismjs/themes/prism-tomorrow.css';
// Or directly import it in your main JS/TS file if your build system supports it.

interface SmartContentRendererProps {
  content: string;
}

const SmartContentRenderer: React.FC<SmartContentRendererProps> = ({ content }) => {
  // Detect content format
  const detectFormat = (text: string): 'markdown' | 'json' | 'xml' | 'html' | 'code' | 'text' => {
    const trimmed = text.trim();
    
    // Detect JSON
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
      try {
        JSON.parse(trimmed);
        return 'json';
      } catch {
        // Not valid JSON
      }
    }
    
    // Detect XML
    if (trimmed.startsWith('<') && trimmed.includes('>') && trimmed.includes('</')) {
      return 'xml';
    }
    
    // Detect HTML
    if (trimmed.startsWith('<html') || trimmed.startsWith('<!DOCTYPE') || 
        (trimmed.startsWith('<') && trimmed.includes('</') && !trimmed.includes('```'))) {
      return 'html';
    }
    
    // Detect fenced code blocks with language identifiers
    if (/^```[a-zA-Z]*\n[\s\S]*\n```$/.test(trimmed)) {
      return 'markdown';
    }
    
    // Detect inline code or code blocks
    if (trimmed.includes('```') || trimmed.includes('`')) {
      return 'markdown';
    }
    
    // Detect Markdown features
    if (trimmed.includes('#') || trimmed.includes('**') || trimmed.includes('*') || 
        trimmed.includes('[') || trimmed.includes('![') || trimmed.includes('- ')) {
      return 'markdown';
    }
    
    // FIX: If no specific format is detected, assume it's Markdown by default.
    // This ensures ReactMarkdown attempts to render even simple text,
    // which might contain subtle Markdown or just benefit from ReactMarkdown's default rendering.
    return 'markdown'; // Changed from 'text'
  };

  const format = detectFormat(content);

  // Render JSON
  if (format === 'json') {
    try {
      const parsed = JSON.parse(content.trim());
      return (
        <div className="json-renderer">
          <SyntaxHighlighter
            language="json"
            style={tomorrow}
            customStyle={{
              margin: 0,
              borderRadius: '8px',
              fontSize: '14px',
              lineHeight: '1.5'
            }}
          >
            {JSON.stringify(parsed, null, 2)}
          </SyntaxHighlighter>
        </div>
      );
    } catch {
      // If parsing fails, fallback to plain text
    }
  }

  // Render XML
  if (format === 'xml') {
    return (
      <div className="xml-renderer">
        <SyntaxHighlighter
          language="xml"
          style={tomorrow}
          customStyle={{
            margin: 0,
            borderRadius: '8px',
            fontSize: '14px',
            lineHeight: '1.5'
          }}
        >
          {content}
        </SyntaxHighlighter>
      </div>
    );
  }

  // Render HTML
  if (format === 'html') {
    return (
      <div className="html-renderer">
        <div className="html-preview">
          <div 
            className="html-content"
            dangerouslySetInnerHTML={{ __html: content }}
          />
        </div>
        <details className="html-source">
          <summary>View Source Code</summary>
          <SyntaxHighlighter
            language="html"
            style={tomorrow}
            customStyle={{
              margin: 0,
              borderRadius: '8px',
              fontSize: '14px',
              lineHeight: '1.5'
            }}
          >
            {content}
          </SyntaxHighlighter>
        </details>
      </div>
    );
  }

  // Render Markdown
  if (format === 'markdown') {
    // Process content to remove leading/trailing code block markers with language identifiers
    let processedContent = content;
    const codeBlockRegex = /^```[a-zA-Z]*\n([\s\S]*?)\n```$/;
    const match = content.trim().match(codeBlockRegex);
    if (match) {
      processedContent = match[1];
    }
    
    return (
      <div className="markdown-renderer">
        <ReactMarkdown
          rehypePlugins={[rehypeHighlight, rehypeRaw]}
          remarkPlugins={[remarkGfm]}
          components={{
            code(props: any) {
              const { node, inline, className, children, ...rest } = props;
              const match = /language-(\w+)/.exec(className || '');
              return !inline && match ? (
                <SyntaxHighlighter
                  style={tomorrow}
                  language={match[1]}
                  PreTag="div"
                  customStyle={{
                    margin: '8px 0',
                    borderRadius: '8px',
                    fontSize: '14px',
                    lineHeight: '1.5'
                  }}
                  {...rest}
                >
                  {String(children).replace(/\n$/, '')}
                </SyntaxHighlighter>
              ) : (
                <code className={className} {...rest}>
                  {children}
                </code>
              );
            },
            table(props: any) {
              const { children } = props;
              return (
                <div className="table-container">
                  <table className="markdown-table">
                    {children}
                  </table>
                </div>
              );
            },
            // Add styling for other markdown elements
            h1: ({node, ...props}) => <h1 className="text-3xl font-bold mt-6 mb-4 text-white">{props.children}</h1>,
            h2: ({node, ...props}) => <h2 className="text-2xl font-bold mt-5 mb-3 text-white">{props.children}</h2>,
            h3: ({node, ...props}) => <h3 className="text-xl font-bold mt-4 mb-2 text-white">{props.children}</h3>,
            p: ({node, ...props}) => <p className="my-3">{props.children}</p>,
            ul: ({node, ...props}) => <ul className="list-disc list-inside my-2">{props.children}</ul>,
            ol: ({node, ...props}) => <ol className="list-decimal list-inside my-2">{props.children}</ol>,
            li: ({node, ...props}) => <li className="my-1">{props.children}</li>,
            a: ({node, ...props}) => <a className="text-blue-400 hover:underline" {...props} />,
            blockquote: ({node, ...props}) => <blockquote className="border-l-4 border-gray-500 pl-4 italic my-4 text-gray-300">{props.children}</blockquote>,
          }}
        >
          {processedContent}
        </ReactMarkdown>
      </div>
    );
  }

  // Render plain text
  return (
    <div className="text-renderer">
      <pre className="text-content">{content}</pre>
    </div>
  );
};

export default SmartContentRenderer;