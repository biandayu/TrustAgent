import React from 'react';
import ReactMarkdown from 'react-markdown';
import rehypeHighlight from 'rehype-highlight';
import rehypeRaw from 'rehype-raw';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { tomorrow } from 'react-syntax-highlighter/dist/esm/styles/prism';

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
    
    // Detect code blocks
    if (trimmed.includes('```') || trimmed.includes('`')) {
      return 'markdown';
    }
    
    // Detect Markdown features
    if (trimmed.includes('#') || trimmed.includes('**') || trimmed.includes('*') || 
        trimmed.includes('[') || trimmed.includes('![') || trimmed.includes('- ')) {
      return 'markdown';
    }
    
    return 'text';
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
            }
          }}
        >
          {content}
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
