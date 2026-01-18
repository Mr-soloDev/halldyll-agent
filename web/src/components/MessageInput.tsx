import { useState, useRef, useEffect, type KeyboardEvent } from 'react'
import './MessageInput.css'

interface MessageInputProps {
  onSend: (message: string) => void
  isLoading: boolean
}

export function MessageInput({ onSend, isLoading }: MessageInputProps) {
  const [value, setValue] = useState('')
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto'
      textareaRef.current.style.height = Math.min(textareaRef.current.scrollHeight, 200) + 'px'
    }
  }, [value])

  const handleSubmit = () => {
    if (value.trim() && !isLoading) {
      onSend(value)
      setValue('')
    }
  }

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSubmit()
    }
  }

  return (
    <div className="input-container">
      <div className="input-wrapper">
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Message Halldyll..."
          rows={1}
          disabled={isLoading}
        />
        <button
          className={`send-btn ${value.trim() && !isLoading ? 'active' : ''}`}
          onClick={handleSubmit}
          disabled={!value.trim() || isLoading}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
            <path
              d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </button>
      </div>
      <p className="disclaimer">
        Halldyll can make mistakes. Please verify important information.
      </p>
    </div>
  )
}
