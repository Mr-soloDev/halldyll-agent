import { useState, useRef, useCallback, useEffect } from "react";
import { useOllama } from "../../hooks/useOllama";
import "./Composer.css";

interface ComposerProps {
  onSend: (content: string) => Promise<void>;
  disabled?: boolean;
}

export function Composer({ onSend, disabled = false }: ComposerProps) {
  const [input, setInput] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const { ollamaStatus } = useOllama();

  const canSend =
    input.trim().length > 0 &&
    !disabled &&
    ollamaStatus === "ready";

  const adjustHeight = useCallback(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = "auto";
      textarea.style.height = `${Math.min(textarea.scrollHeight, 200)}px`;
    }
  }, []);

  useEffect(() => {
    adjustHeight();
  }, [input, adjustHeight]);

  const handleSubmit = useCallback(async () => {
    if (!canSend) return;
    const message = input.trim();
    setInput("");
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
    await onSend(message);
  }, [canSend, input, onSend]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
      }
    },
    [handleSubmit]
  );

  const handleInput = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      setInput(e.target.value);
    },
    []
  );

  const getPlaceholder = () => {
    if (ollamaStatus !== "ready") {
      return "En attente d'Ollama...";
    }
    if (disabled) {
      return "Generation en cours...";
    }
    return "Ecrivez votre message...";
  };

  return (
    <div className="composer-dock">
      <div className="composer-inner">
        <div className="composer-box">
          <button
            type="button"
            className="composer-attach"
            disabled={disabled}
            title="Joindre un fichier (bientot)"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
              <path
                d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>

          <textarea
            ref={textareaRef}
            className="composer-textarea"
            value={input}
            onChange={handleInput}
            onKeyDown={handleKeyDown}
            placeholder={getPlaceholder()}
            disabled={disabled || ollamaStatus !== "ready"}
            rows={1}
          />

          <button
            type="button"
            className={`composer-send ${canSend ? "active" : ""}`}
            onClick={handleSubmit}
            disabled={!canSend}
            title="Envoyer (Entree)"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
              <path
                d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
        </div>

        <div className="composer-hint">
          Entree pour envoyer, Maj+Entree pour nouvelle ligne
        </div>
      </div>
    </div>
  );
}
