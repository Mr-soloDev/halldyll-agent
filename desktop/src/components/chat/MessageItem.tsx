import type { ChatMessage } from "../../context/types";
import { MarkdownRenderer } from "../common/MarkdownRenderer";
import "./MessageItem.css";

interface MessageItemProps {
  message: ChatMessage;
}

export function MessageItem({ message }: MessageItemProps) {
  const isUser = message.role === "user";

  return (
    <article className={`message ${isUser ? "message-user" : "message-assistant"}`}>
      <div className="message-avatar" aria-hidden="true">
        {isUser ? "U" : "H"}
      </div>
      <div className="message-body">
        <div className="message-role">{isUser ? "Vous" : "Halldyll"}</div>
        <div className="message-content">
          {isUser ? (
            <p>{message.content}</p>
          ) : (
            <MarkdownRenderer content={message.content} />
          )}
        </div>
      </div>
    </article>
  );
}
