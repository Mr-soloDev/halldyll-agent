import { useEffect } from "react";
import { useConversations } from "../../hooks/useConversations";
import { useAppState } from "../../context/AppContext";
import "./Sidebar.css";

interface SidebarProps {
  collapsed: boolean;
  onToggle: () => void;
}

export function Sidebar({ collapsed }: SidebarProps) {
  const {
    conversations,
    activeConversationId,
    loadConversations,
    createNewConversation,
    selectConversation,
    deleteConversation,
  } = useConversations();

  const { isSending } = useAppState();

  useEffect(() => {
    loadConversations();
  }, [loadConversations]);

  const handleNewChat = async () => {
    if (isSending) return;
    await createNewConversation();
  };

  const handleSelect = async (id: string) => {
    if (isSending || id === activeConversationId) return;
    await selectConversation(id);
  };

  const handleDelete = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    if (isSending) return;
    if (confirm("Supprimer cette conversation ?")) {
      await deleteConversation(id);
    }
  };

  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp);
    const now = new Date();
    const diffDays = Math.floor(
      (now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24)
    );

    if (diffDays === 0) return "Aujourd'hui";
    if (diffDays === 1) return "Hier";
    if (diffDays < 7) return `Il y a ${diffDays} jours`;
    return date.toLocaleDateString("fr-FR", { day: "numeric", month: "short" });
  };

  if (collapsed) {
    return null;
  }

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <button
          type="button"
          className="new-chat-btn"
          onClick={handleNewChat}
          disabled={isSending}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
            <path
              d="M12 5v14M5 12h14"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
            />
          </svg>
          Nouvelle conversation
        </button>
      </div>

      <div className="sidebar-list">
        {conversations.length === 0 ? (
          <div className="sidebar-empty">
            Aucune conversation
          </div>
        ) : (
          conversations.map((conv) => (
            <div
              key={conv.id}
              className={`conversation-item ${
                conv.id === activeConversationId ? "active" : ""
              }`}
              onClick={() => handleSelect(conv.id)}
            >
              <div className="conversation-icon">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                  <path
                    d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2v10z"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
              </div>
              <div className="conversation-content">
                <div className="conversation-title">
                  {conv.title || "Nouvelle conversation"}
                </div>
                <div className="conversation-meta">
                  {formatDate(conv.updatedAt)}
                </div>
              </div>
              <button
                type="button"
                className="conversation-delete"
                onClick={(e) => handleDelete(e, conv.id)}
                title="Supprimer"
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
                  <path
                    d="M18 6L6 18M6 6l12 12"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                  />
                </svg>
              </button>
            </div>
          ))
        )}
      </div>

      <div className="sidebar-footer">
        <div className="sidebar-brand">
          <span className="brand-icon">H</span>
          <span className="brand-text">Halldyll</span>
        </div>
      </div>
    </aside>
  );
}
