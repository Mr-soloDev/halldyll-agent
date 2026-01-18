import type { ChatMessage } from "../../context/types";
import { MessageItem } from "./MessageItem";

interface MessageListProps {
  messages: ChatMessage[];
}

export function MessageList({ messages }: MessageListProps) {
  return (
    <div className="message-list" role="log" aria-label="Conversation">
      {messages.map((message) => (
        <MessageItem key={message.id} message={message} />
      ))}
    </div>
  );
}
