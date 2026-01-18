// Typed wrappers for Tauri commands
import { invoke } from "@tauri-apps/api/core";
import type { Conversation, ChatMessage } from "../context/types";

// Backend types matching Rust structs
interface ConversationMeta {
  id: string;
  title: string;
  created_at: number;
  updated_at: number;
  message_count: number;
}

interface ConversationMessage {
  role: string;
  content: string;
  timestamp: number;
}

// Convert backend types to frontend types
function toConversation(meta: ConversationMeta): Conversation {
  return {
    id: meta.id,
    title: meta.title || "Nouvelle conversation",
    createdAt: meta.created_at,
    updatedAt: meta.updated_at,
    messageCount: meta.message_count,
  };
}

function toChatMessage(msg: ConversationMessage, index: number): ChatMessage {
  return {
    id: `msg-${msg.timestamp}-${index}`,
    role: msg.role as "user" | "assistant",
    content: msg.content,
    timestamp: msg.timestamp,
  };
}

// Ollama commands
export async function startOllama(): Promise<void> {
  await invoke("start_ollama_ministral");
}

export async function generateText(
  model: string,
  prompt: string
): Promise<string> {
  return await invoke<string>("ollama_generate_8192", { model, prompt });
}

// Chat commands
export async function chatWithMemory(userMessage: string): Promise<string> {
  return await invoke<string>("chat_with_memory", { userMessage });
}

// Conversation commands
export async function listConversations(): Promise<Conversation[]> {
  const metas = await invoke<ConversationMeta[]>("list_conversations");
  return metas.map(toConversation);
}

export async function createConversation(): Promise<Conversation> {
  const meta = await invoke<ConversationMeta>("create_conversation");
  return toConversation(meta);
}

export async function switchConversation(conversationId: string): Promise<void> {
  await invoke("switch_conversation", { conversationId });
}

export async function deleteConversation(conversationId: string): Promise<void> {
  await invoke("delete_conversation", { conversationId });
}

export async function renameConversation(
  conversationId: string,
  title: string
): Promise<void> {
  await invoke("rename_conversation", { conversationId, title });
}

export async function loadConversationMessages(
  conversationId: string,
  limit: number = 100
): Promise<ChatMessage[]> {
  const messages = await invoke<ConversationMessage[]>(
    "load_conversation_messages",
    { conversationId, limit }
  );
  return messages.map(toChatMessage);
}

export async function getActiveConversation(): Promise<string | null> {
  return await invoke<string | null>("get_active_conversation");
}

export async function generateConversationTitle(
  conversationId: string,
  firstUserMessage: string,
  firstAssistantMessage: string
): Promise<string> {
  return await invoke<string>("generate_conversation_title", {
    conversationId,
    firstUserMessage,
    firstAssistantMessage,
  });
}
