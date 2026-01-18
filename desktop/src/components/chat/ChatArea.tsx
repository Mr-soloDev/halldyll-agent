import { useEffect, useRef } from "react";
import { useChat } from "../../hooks/useChat";
import { MessageList } from "./MessageList";
import { TypingIndicator } from "./TypingIndicator";
import { EmptyState } from "./EmptyState";
import { Composer } from "../composer/Composer";
import { useConversations } from "../../hooks/useConversations";
import "./ChatArea.css";

export function ChatArea() {
  const { messages, isSending, activeConversationId, sendMessage } = useChat();
  const { createNewConversation, updateConversationTitle } = useConversations();
  const scrollRef = useRef<HTMLDivElement>(null);
  const isFirstMessage = useRef(true);

  // Track if this is the first message in conversation
  useEffect(() => {
    isFirstMessage.current = messages.length === 0;
  }, [activeConversationId]);

  // Auto-scroll on new messages
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages, isSending]);

  const handleSend = async (content: string) => {
    // If no active conversation, create one first
    let convId = activeConversationId;
    if (!convId) {
      const newConv = await createNewConversation();
      if (!newConv) return;
      convId = newConv.id;
    }

    const wasFirst = isFirstMessage.current;
    const result = await sendMessage(content);

    // Generate title after first exchange
    if (wasFirst && result && convId) {
      isFirstMessage.current = false;
      // Generate title in background
      setTimeout(() => {
        updateConversationTitle(convId!, result.userMsg.content, result.assistantMsg.content);
      }, 500);
    }
  };

  return (
    <div className="chat-area">
      <div className="chat-scroll" ref={scrollRef}>
        <div className="chat-inner">
          {messages.length === 0 && !isSending ? (
            <EmptyState />
          ) : (
            <>
              <MessageList messages={messages} />
              {isSending && <TypingIndicator />}
            </>
          )}
        </div>
      </div>
      <Composer onSend={handleSend} disabled={isSending} />
    </div>
  );
}
