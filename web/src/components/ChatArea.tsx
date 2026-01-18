import { useRef, useEffect } from 'react'
import { Logo } from './Logo'
import type { Message } from '../types'
import './ChatArea.css'

interface ChatAreaProps {
  messages: Message[]
  isLoading: boolean
  onMenuClick: () => void
}

export function ChatArea({ messages, isLoading, onMenuClick }: ChatAreaProps) {
  const bottomRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages, isLoading])

  if (messages.length === 0 && !isLoading) {
    return (
      <div className="chat-area">
        <Header onMenuClick={onMenuClick} />
        <div className="welcome-screen">
          <Logo size={64} />
          <h1>Hello!</h1>
          <p>I'm Halldyll, your AI assistant. How can I help you today?</p>
        </div>
      </div>
    )
  }

  return (
    <div className="chat-area">
      <Header onMenuClick={onMenuClick} />
      <div className="messages-container">
        <div className="messages-list">
          {messages.map(message => (
            <MessageBubble key={message.id} message={message} />
          ))}
          {isLoading && <TypingIndicator />}
          <div ref={bottomRef} />
        </div>
      </div>
    </div>
  )
}

function Header({ onMenuClick }: { onMenuClick: () => void }) {
  return (
    <header className="chat-header">
      <button className="menu-btn" onClick={onMenuClick}>
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M3 12h18M3 6h18M3 18h18" />
        </svg>
      </button>
      <div className="header-title">
        <Logo size={24} />
        <span>Halldyll</span>
      </div>
      <div className="header-spacer" />
    </header>
  )
}

function MessageBubble({ message }: { message: Message }) {
  const isUser = message.role === 'user'

  return (
    <div className={`message ${isUser ? 'user' : 'assistant'}`}>
      {!isUser && (
        <div className="message-avatar">
          <Logo size={28} />
        </div>
      )}
      <div className="message-content">
        <p>{message.content}</p>
      </div>
    </div>
  )
}

function TypingIndicator() {
  return (
    <div className="message assistant">
      <div className="message-avatar">
        <Logo size={28} />
      </div>
      <div className="message-content typing">
        <span className="dot" />
        <span className="dot" />
        <span className="dot" />
      </div>
    </div>
  )
}
