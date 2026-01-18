// Hook for chat messaging
import { useCallback } from "react";
import { useAppDispatch, useAppState } from "../context/AppContext";
import * as tauri from "../services/tauri";
import type { ChatMessage } from "../context/types";

export function useChat() {
  const dispatch = useAppDispatch();
  const { messages, isSending, activeConversationId } = useAppState();

  const sendMessage = useCallback(
    async (content: string): Promise<{ userMsg: ChatMessage; assistantMsg: ChatMessage } | null> => {
      if (!content.trim() || isSending) return null;

      dispatch({ type: "SET_SENDING", isSending: true });

      // Create user message
      const userMsg: ChatMessage = {
        id: `user-${Date.now()}`,
        role: "user",
        content: content.trim(),
        timestamp: Date.now(),
      };

      dispatch({ type: "ADD_MESSAGE", message: userMsg });

      try {
        // Call backend
        const response = await tauri.chatWithMemory(content.trim());

        // Create assistant message
        const assistantMsg: ChatMessage = {
          id: `assistant-${Date.now()}`,
          role: "assistant",
          content: response,
          timestamp: Date.now(),
        };

        dispatch({ type: "ADD_MESSAGE", message: assistantMsg });
        dispatch({ type: "SET_SENDING", isSending: false });

        return { userMsg, assistantMsg };
      } catch (error) {
        console.error("Failed to send message:", error);
        dispatch({ type: "SET_SENDING", isSending: false });
        return null;
      }
    },
    [dispatch, isSending]
  );

  const loadMessages = useCallback(
    async (conversationId: string) => {
      try {
        const msgs = await tauri.loadConversationMessages(conversationId);
        dispatch({ type: "SET_MESSAGES", messages: msgs });
      } catch (error) {
        console.error("Failed to load messages:", error);
      }
    },
    [dispatch]
  );

  const clearMessages = useCallback(() => {
    dispatch({ type: "SET_MESSAGES", messages: [] });
  }, [dispatch]);

  return {
    messages,
    isSending,
    activeConversationId,
    sendMessage,
    loadMessages,
    clearMessages,
  };
}
