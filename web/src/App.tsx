import { useState } from 'react'
import { useChat } from './hooks/useChat'
import { Sidebar } from './components/Sidebar'
import { ChatArea } from './components/ChatArea'
import { MessageInput } from './components/MessageInput'
import './App.css'

function App() {
  const [sidebarOpen, setSidebarOpen] = useState(false)

  const {
    conversations,
    activeConversation,
    activeId,
    isLoading,
    setActiveId,
    createConversation,
    deleteConversation,
    sendMessage
  } = useChat()

  return (
    <div className="app">
      <Sidebar
        conversations={conversations}
        activeId={activeId}
        onSelect={setActiveId}
        onCreate={createConversation}
        onDelete={deleteConversation}
        isOpen={sidebarOpen}
        onClose={() => setSidebarOpen(false)}
      />

      <main className="main-content">
        <ChatArea
          messages={activeConversation?.messages || []}
          isLoading={isLoading}
          onMenuClick={() => setSidebarOpen(true)}
        />

        <MessageInput
          onSend={sendMessage}
          isLoading={isLoading}
        />
      </main>
    </div>
  )
}

export default App
