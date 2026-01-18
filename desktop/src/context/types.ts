// Types for the application state

export interface Conversation {
  id: string;
  title: string;
  createdAt: number;
  updatedAt: number;
  messageCount: number;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
}

export type OllamaStatus = "starting" | "ready" | "error";

export interface AppState {
  // Ollama status
  ollamaStatus: OllamaStatus;
  ollamaStatusText: string;

  // Conversations
  conversations: Conversation[];
  activeConversationId: string | null;

  // Current chat
  messages: ChatMessage[];
  isSending: boolean;

  // UI state
  sidebarCollapsed: boolean;
}

export type AppAction =
  | { type: "SET_OLLAMA_STATUS"; status: OllamaStatus; text: string }
  | { type: "SET_CONVERSATIONS"; conversations: Conversation[] }
  | { type: "SELECT_CONVERSATION"; id: string | null }
  | { type: "ADD_CONVERSATION"; conversation: Conversation }
  | { type: "UPDATE_CONVERSATION"; id: string; updates: Partial<Conversation> }
  | { type: "DELETE_CONVERSATION"; id: string }
  | { type: "SET_MESSAGES"; messages: ChatMessage[] }
  | { type: "ADD_MESSAGE"; message: ChatMessage }
  | { type: "SET_SENDING"; isSending: boolean }
  | { type: "TOGGLE_SIDEBAR" };

export const initialState: AppState = {
  ollamaStatus: "starting",
  ollamaStatusText: "Initialisation...",
  conversations: [],
  activeConversationId: null,
  messages: [],
  isSending: false,
  sidebarCollapsed: false,
};

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "SET_OLLAMA_STATUS":
      return {
        ...state,
        ollamaStatus: action.status,
        ollamaStatusText: action.text,
      };

    case "SET_CONVERSATIONS":
      return {
        ...state,
        conversations: action.conversations,
      };

    case "SELECT_CONVERSATION":
      return {
        ...state,
        activeConversationId: action.id,
        messages: [], // Clear messages when switching
      };

    case "ADD_CONVERSATION":
      return {
        ...state,
        conversations: [action.conversation, ...state.conversations],
        activeConversationId: action.conversation.id,
        messages: [],
      };

    case "UPDATE_CONVERSATION":
      return {
        ...state,
        conversations: state.conversations.map((c) =>
          c.id === action.id ? { ...c, ...action.updates } : c
        ),
      };

    case "DELETE_CONVERSATION":
      return {
        ...state,
        conversations: state.conversations.filter((c) => c.id !== action.id),
        activeConversationId:
          state.activeConversationId === action.id
            ? null
            : state.activeConversationId,
        messages:
          state.activeConversationId === action.id ? [] : state.messages,
      };

    case "SET_MESSAGES":
      return {
        ...state,
        messages: action.messages,
      };

    case "ADD_MESSAGE":
      return {
        ...state,
        messages: [...state.messages, action.message],
      };

    case "SET_SENDING":
      return {
        ...state,
        isSending: action.isSending,
      };

    case "TOGGLE_SIDEBAR":
      return {
        ...state,
        sidebarCollapsed: !state.sidebarCollapsed,
      };

    default:
      return state;
  }
}
