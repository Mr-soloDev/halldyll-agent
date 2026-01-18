// Hook for conversation management
import { useCallback } from "react";
import { useAppDispatch, useAppState } from "../context/AppContext";
import * as tauri from "../services/tauri";
import type { Conversation } from "../context/types";

export function useConversations() {
  const dispatch = useAppDispatch();
  const { conversations, activeConversationId } = useAppState();

  const loadConversations = useCallback(async () => {
    try {
      const convs = await tauri.listConversations();
      dispatch({ type: "SET_CONVERSATIONS", conversations: convs });
    } catch (error) {
      console.error("Failed to load conversations:", error);
    }
  }, [dispatch]);

  const createNewConversation = useCallback(async (): Promise<Conversation | null> => {
    try {
      const conv = await tauri.createConversation();
      dispatch({ type: "ADD_CONVERSATION", conversation: conv });
      return conv;
    } catch (error) {
      console.error("Failed to create conversation:", error);
      return null;
    }
  }, [dispatch]);

  const selectConversation = useCallback(
    async (id: string) => {
      try {
        await tauri.switchConversation(id);
        dispatch({ type: "SELECT_CONVERSATION", id });

        // Load messages for the conversation
        const messages = await tauri.loadConversationMessages(id);
        dispatch({ type: "SET_MESSAGES", messages });
      } catch (error) {
        console.error("Failed to switch conversation:", error);
      }
    },
    [dispatch]
  );

  const deleteConversation = useCallback(
    async (id: string) => {
      try {
        await tauri.deleteConversation(id);
        dispatch({ type: "DELETE_CONVERSATION", id });
      } catch (error) {
        console.error("Failed to delete conversation:", error);
      }
    },
    [dispatch]
  );

  const renameConversation = useCallback(
    async (id: string, title: string) => {
      try {
        await tauri.renameConversation(id, title);
        dispatch({
          type: "UPDATE_CONVERSATION",
          id,
          updates: { title },
        });
      } catch (error) {
        console.error("Failed to rename conversation:", error);
      }
    },
    [dispatch]
  );

  const updateConversationTitle = useCallback(
    async (
      id: string,
      firstUserMessage: string,
      firstAssistantMessage: string
    ) => {
      try {
        const title = await tauri.generateConversationTitle(
          id,
          firstUserMessage,
          firstAssistantMessage
        );
        dispatch({
          type: "UPDATE_CONVERSATION",
          id,
          updates: { title },
        });
      } catch (error) {
        console.error("Failed to generate title:", error);
      }
    },
    [dispatch]
  );

  return {
    conversations,
    activeConversationId,
    loadConversations,
    createNewConversation,
    selectConversation,
    deleteConversation,
    renameConversation,
    updateConversationTitle,
  };
}
