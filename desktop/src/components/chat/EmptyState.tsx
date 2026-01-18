import "./EmptyState.css";

export function EmptyState() {
  return (
    <div className="empty-state">
      <div className="empty-icon">H</div>
      <h2 className="empty-title">Bienvenue sur Halldyll</h2>
      <p className="empty-text">
        Je suis ton assistant IA local. Pose-moi une question pour commencer.
      </p>
      <div className="empty-hints">
        <div className="hint">Entree pour envoyer</div>
        <div className="hint">Maj+Entree pour nouvelle ligne</div>
      </div>
    </div>
  );
}
