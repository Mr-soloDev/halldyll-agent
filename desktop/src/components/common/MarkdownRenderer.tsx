import { useMemo } from "react";
import "./MarkdownRenderer.css";

interface MarkdownRendererProps {
  content: string;
}

interface ParsedBlock {
  type: "paragraph" | "code" | "heading" | "list" | "blockquote";
  content: string;
  language?: string;
  level?: number;
}

export function MarkdownRenderer({ content }: MarkdownRendererProps) {
  const blocks = useMemo(() => parseMarkdown(content), [content]);

  return (
    <div className="markdown-content">
      {blocks.map((block, index) => renderBlock(block, index))}
    </div>
  );
}

function parseMarkdown(text: string): ParsedBlock[] {
  const blocks: ParsedBlock[] = [];
  const lines = text.split("\n");
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Code block (```)
    if (line.startsWith("```")) {
      const language = line.slice(3).trim();
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) {
        codeLines.push(lines[i]);
        i++;
      }
      blocks.push({
        type: "code",
        content: codeLines.join("\n"),
        language: language || undefined,
      });
      i++;
      continue;
    }

    // Heading (# ## ###)
    const headingMatch = line.match(/^(#{1,6})\s+(.+)$/);
    if (headingMatch) {
      blocks.push({
        type: "heading",
        content: headingMatch[2],
        level: headingMatch[1].length,
      });
      i++;
      continue;
    }

    // Blockquote (>)
    if (line.startsWith("> ")) {
      const quoteLines: string[] = [line.slice(2)];
      i++;
      while (i < lines.length && lines[i].startsWith("> ")) {
        quoteLines.push(lines[i].slice(2));
        i++;
      }
      blocks.push({
        type: "blockquote",
        content: quoteLines.join("\n"),
      });
      continue;
    }

    // Unordered list (- or *)
    if (line.match(/^[-*]\s+/)) {
      const listItems: string[] = [];
      while (i < lines.length && lines[i].match(/^[-*]\s+/)) {
        listItems.push(lines[i].replace(/^[-*]\s+/, ""));
        i++;
      }
      blocks.push({
        type: "list",
        content: listItems.join("\n"),
      });
      continue;
    }

    // Ordered list (1. 2. etc)
    if (line.match(/^\d+\.\s+/)) {
      const listItems: string[] = [];
      while (i < lines.length && lines[i].match(/^\d+\.\s+/)) {
        listItems.push(lines[i].replace(/^\d+\.\s+/, ""));
        i++;
      }
      blocks.push({
        type: "list",
        content: listItems.join("\n"),
      });
      continue;
    }

    // Empty line - skip
    if (line.trim() === "") {
      i++;
      continue;
    }

    // Paragraph - collect consecutive non-empty lines
    const paragraphLines: string[] = [line];
    i++;
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !lines[i].startsWith("```") &&
      !lines[i].startsWith("#") &&
      !lines[i].startsWith("> ") &&
      !lines[i].match(/^[-*]\s+/) &&
      !lines[i].match(/^\d+\.\s+/)
    ) {
      paragraphLines.push(lines[i]);
      i++;
    }
    blocks.push({
      type: "paragraph",
      content: paragraphLines.join(" "),
    });
  }

  return blocks;
}

function renderBlock(block: ParsedBlock, key: number): JSX.Element {
  switch (block.type) {
    case "code":
      return (
        <pre key={key} className="md-code-block">
          {block.language && (
            <span className="md-code-lang">{block.language}</span>
          )}
          <code>{block.content}</code>
        </pre>
      );

    case "heading": {
      const Tag = `h${Math.min(block.level || 1, 6)}` as keyof JSX.IntrinsicElements;
      return (
        <Tag key={key} className="md-heading">
          {renderInline(block.content)}
        </Tag>
      );
    }

    case "blockquote":
      return (
        <blockquote key={key} className="md-blockquote">
          {renderInline(block.content)}
        </blockquote>
      );

    case "list":
      return (
        <ul key={key} className="md-list">
          {block.content.split("\n").map((item, i) => (
            <li key={i}>{renderInline(item)}</li>
          ))}
        </ul>
      );

    case "paragraph":
    default:
      return (
        <p key={key} className="md-paragraph">
          {renderInline(block.content)}
        </p>
      );
  }
}

function renderInline(text: string): (string | JSX.Element)[] {
  const elements: (string | JSX.Element)[] = [];
  let remaining = text;
  let keyCounter = 0;

  while (remaining.length > 0) {
    // Inline code (`code`)
    const codeMatch = remaining.match(/^`([^`]+)`/);
    if (codeMatch) {
      elements.push(
        <code key={keyCounter++} className="md-inline-code">
          {codeMatch[1]}
        </code>
      );
      remaining = remaining.slice(codeMatch[0].length);
      continue;
    }

    // Bold (**text**)
    const boldMatch = remaining.match(/^\*\*([^*]+)\*\*/);
    if (boldMatch) {
      elements.push(
        <strong key={keyCounter++}>{boldMatch[1]}</strong>
      );
      remaining = remaining.slice(boldMatch[0].length);
      continue;
    }

    // Italic (*text* or _text_)
    const italicMatch = remaining.match(/^[*_]([^*_]+)[*_]/);
    if (italicMatch) {
      elements.push(
        <em key={keyCounter++}>{italicMatch[1]}</em>
      );
      remaining = remaining.slice(italicMatch[0].length);
      continue;
    }

    // Link [text](url)
    const linkMatch = remaining.match(/^\[([^\]]+)\]\(([^)]+)\)/);
    if (linkMatch) {
      elements.push(
        <a
          key={keyCounter++}
          href={linkMatch[2]}
          target="_blank"
          rel="noopener noreferrer"
          className="md-link"
        >
          {linkMatch[1]}
        </a>
      );
      remaining = remaining.slice(linkMatch[0].length);
      continue;
    }

    // Regular text - find next special character or end
    const nextSpecial = remaining.search(/[`*_\[]/);
    if (nextSpecial === -1) {
      elements.push(remaining);
      break;
    } else if (nextSpecial === 0) {
      // Special char that didn't match patterns - treat as literal
      elements.push(remaining[0]);
      remaining = remaining.slice(1);
    } else {
      elements.push(remaining.slice(0, nextSpecial));
      remaining = remaining.slice(nextSpecial);
    }
  }

  return elements;
}
