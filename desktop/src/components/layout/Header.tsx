import { useOllama } from "../../hooks/useOllama";
import "./Header.css";

interface HeaderProps {
  onToggleSidebar: () => void;
  sidebarCollapsed: boolean;
}

export function Header({ onToggleSidebar, sidebarCollapsed }: HeaderProps) {
  const { ollamaStatus, ollamaStatusText, initializeOllama } = useOllama();

  const statusVariant =
    ollamaStatus === "ready" ? "ok" : ollamaStatus === "starting" ? "warn" : "err";

  const statusLabel =
    ollamaStatus === "ready"
      ? "Pret"
      : ollamaStatus === "starting"
      ? "Demarrage"
      : "Erreur";

  return (
    <header className="header">
      <div className="header-left">
        <button
          className="sidebar-toggle"
          onClick={onToggleSidebar}
          title={sidebarCollapsed ? "Ouvrir sidebar" : "Fermer sidebar"}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
            <path
              d="M3 12h18M3 6h18M3 18h18"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
            />
          </svg>
        </button>
        <div className="header-title">Halldyll</div>
      </div>

      <div className="header-right">
        <div className={`status-pill ${statusVariant}`} title={ollamaStatusText}>
          <span className="status-dot" />
          <span className="status-text">{statusLabel}</span>
        </div>

        {ollamaStatus === "error" && (
          <button className="retry-btn" onClick={initializeOllama}>
            Relancer
          </button>
        )}
      </div>
    </header>
  );
}
