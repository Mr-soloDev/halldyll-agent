import "./TypingIndicator.css";

export function TypingIndicator() {
  return (
    <article className="message message-assistant">
      <div className="message-avatar" aria-hidden="true">
        H
      </div>
      <div className="message-body">
        <div className="message-role">Halldyll</div>
        <div className="typing-indicator" aria-label="Generation en cours">
          <span className="typing-dot" />
          <span className="typing-dot" />
          <span className="typing-dot" />
          <span className="typing-text">Generation...</span>
        </div>
      </div>
    </article>
  );
}
