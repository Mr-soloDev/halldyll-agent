// Hook for Ollama status management
import { useCallback } from "react";
import { useAppDispatch, useAppState } from "../context/AppContext";
import * as tauri from "../services/tauri";

export function useOllama() {
  const dispatch = useAppDispatch();
  const { ollamaStatus, ollamaStatusText } = useAppState();

  const initializeOllama = useCallback(async () => {
    dispatch({
      type: "SET_OLLAMA_STATUS",
      status: "starting",
      text: "Demarrage d'Ollama...",
    });

    try {
      await tauri.startOllama();
      dispatch({
        type: "SET_OLLAMA_STATUS",
        status: "ready",
        text: "Pret",
      });
      return true;
    } catch (error) {
      console.error("Failed to start Ollama:", error);
      dispatch({
        type: "SET_OLLAMA_STATUS",
        status: "error",
        text: `Erreur: ${error}`,
      });
      return false;
    }
  }, [dispatch]);

  return {
    ollamaStatus,
    ollamaStatusText,
    initializeOllama,
    isReady: ollamaStatus === "ready",
    isLoading: ollamaStatus === "starting",
    hasError: ollamaStatus === "error",
  };
}
