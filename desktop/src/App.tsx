import { useState, useEffect } from "react";
import { AppProvider } from "./context/AppContext";
import { Sidebar } from "./components/layout/Sidebar";
import { Header } from "./components/layout/Header";
import { ChatArea } from "./components/chat/ChatArea";
import { useOllama } from "./hooks/useOllama";
import "./styles/variables.css";
import "./styles/reset.css";
import "./App.css";

function AppContent() {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const { initializeOllama } = useOllama();

  // Initialize Ollama on app startup
  useEffect(() => {
    initializeOllama();
  }, [initializeOllama]);

  const toggleSidebar = () => {
    setSidebarCollapsed((prev) => !prev);
  };

  return (
    <div className="app-layout">
      <Sidebar collapsed={sidebarCollapsed} onToggle={toggleSidebar} />

      <div className="app-main">
        <Header
          onToggleSidebar={toggleSidebar}
          sidebarCollapsed={sidebarCollapsed}
        />

        <div className="app-content">
          <ChatArea />
        </div>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <AppProvider>
      <AppContent />
    </AppProvider>
  );
}
