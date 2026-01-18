import { useState, useCallback, useEffect } from 'react'
import type { Message, Conversation } from '../types/index'

const STORAGE_KEY = 'halldyll-conversations'

function generateId(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2)
}

function generateTitle(content: string): string {
  const words = content.slice(0, 50).split(' ').slice(0, 6).join(' ')
  return words + (content.length > 50 ? '...' : '')
}

export function useChat() {
  const [conversations, setConversations] = useState<Conversation[]>([])
  const [activeId, setActiveId] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  // Load from localStorage
  useEffect(() => {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (saved) {
      try {
        const parsed = JSON.parse(saved)
        const convs = parsed.map((c: Conversation) => ({
          ...c,
          createdAt: new Date(c.createdAt),
          updatedAt: new Date(c.updatedAt),
          messages: c.messages.map((m: Message) => ({
            ...m,
            timestamp: new Date(m.timestamp)
          }))
        }))
        setConversations(convs)
        if (convs.length > 0) {
          setActiveId(convs[0].id)
        }
      } catch (e) {
        console.error('Failed to load conversations:', e)
      }
    }
  }, [])

  // Save to localStorage
  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(conversations))
  }, [conversations])

  const activeConversation = conversations.find(c => c.id === activeId) || null

  const createConversation = useCallback(() => {
    const newConv: Conversation = {
      id: generateId(),
      title: 'New chat',
      messages: [],
      createdAt: new Date(),
      updatedAt: new Date()
    }
    setConversations(prev => [newConv, ...prev])
    setActiveId(newConv.id)
    return newConv.id
  }, [])

  const deleteConversation = useCallback((id: string) => {
    setConversations(prev => prev.filter(c => c.id !== id))
    if (activeId === id) {
      setActiveId(conversations.length > 1 ? conversations.find(c => c.id !== id)?.id || null : null)
    }
  }, [activeId, conversations])

  const sendMessage = useCallback(async (content: string) => {
    if (!content.trim() || isLoading) return

    let convId = activeId
    if (!convId) {
      convId = createConversation()
    }

    const userMessage: Message = {
      id: generateId(),
      role: 'user',
      content: content.trim(),
      timestamp: new Date()
    }

    // Add user message
    setConversations(prev => prev.map(c => {
      if (c.id !== convId) return c
      const isFirstMessage = c.messages.length === 0
      return {
        ...c,
        title: isFirstMessage ? generateTitle(content) : c.title,
        messages: [...c.messages, userMessage],
        updatedAt: new Date()
      }
    }))

    setIsLoading(true)

    try {
      const response = await fetch('/api/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ message: content.trim() })
      })

      if (!response.ok) {
        throw new Error('Failed to get response')
      }

      const data = await response.json()

      const assistantMessage: Message = {
        id: generateId(),
        role: 'assistant',
        content: data.response,
        timestamp: new Date()
      }

      setConversations(prev => prev.map(c => {
        if (c.id !== convId) return c
        return {
          ...c,
          messages: [...c.messages, assistantMessage],
          updatedAt: new Date()
        }
      }))
    } catch (error) {
      console.error('Chat error:', error)
      const errorMessage: Message = {
        id: generateId(),
        role: 'assistant',
        content: 'Sorry, something went wrong. Please try again.',
        timestamp: new Date()
      }
      setConversations(prev => prev.map(c => {
        if (c.id !== convId) return c
        return {
          ...c,
          messages: [...c.messages, errorMessage],
          updatedAt: new Date()
        }
      }))
    } finally {
      setIsLoading(false)
    }
  }, [activeId, isLoading, createConversation])

  return {
    conversations,
    activeConversation,
    activeId,
    isLoading,
    setActiveId,
    createConversation,
    deleteConversation,
    sendMessage
  }
}
