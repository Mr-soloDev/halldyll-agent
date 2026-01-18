# Halldyll Web Interface

Interface web React style ChatGPT pour l'agent Halldyll.

## Structure des fichiers

```
web/src/
├── main.tsx              # Point d'entrée React
├── App.tsx               # Composant racine + layout
├── App.css               # Styles du layout principal
├── styles/
│   └── index.css         # Variables CSS + styles globaux
├── types/
│   └── index.ts          # Types TypeScript (Message, Conversation)
├── hooks/
│   └── useChat.ts        # Hook de gestion du chat
└── components/
    ├── Logo.tsx          # Logo bonhomme souriant
    ├── Logo.css
    ├── Sidebar.tsx       # Barre latérale conversations
    ├── Sidebar.css
    ├── ChatArea.tsx      # Zone d'affichage des messages
    ├── ChatArea.css
    ├── MessageInput.tsx  # Zone de saisie du message
    └── MessageInput.css
```

## Modules

### `main.tsx`
Point d'entrée de l'application. Monte le composant `App` dans le DOM et importe les styles globaux.

### `App.tsx`
Composant racine qui orchestre l'interface :
- Gère l'état d'ouverture de la sidebar (mobile)
- Utilise le hook `useChat` pour la logique métier
- Assemble les composants Sidebar, ChatArea et MessageInput

### `styles/index.css`
Définit les variables CSS globales :
- Couleurs du thème sombre (`--bg-primary`, `--text-primary`, etc.)
- Espacements (`--sidebar-width`, `--header-height`)
- Transitions et border-radius
- Reset CSS et styles de base (scrollbar, sélection)

### `types/index.ts`
Types TypeScript partagés :
- `Message` : id, role (user/assistant), content, timestamp
- `Conversation` : id, title, messages[], createdAt, updatedAt
- `ChatState` : état global du chat

### `hooks/useChat.ts`
Hook personnalisé qui gère toute la logique du chat :
- **État** : conversations, activeId, isLoading
- **Persistance** : sauvegarde/charge depuis localStorage
- **Actions** :
  - `createConversation()` : crée une nouvelle conversation
  - `deleteConversation(id)` : supprime une conversation
  - `sendMessage(content)` : envoie un message à l'API `/api/chat`
  - `setActiveId(id)` : change la conversation active

### `components/Logo.tsx`
Logo SVG du bonhomme souriant violet :
- Cercle violet (#6366f1) avec yeux et sourire
- Joues roses semi-transparentes
- Prop `size` pour ajuster la taille

### `components/Sidebar.tsx`
Barre latérale gauche (style ChatGPT) :
- Header avec logo et nom "Halldyll"
- Bouton "New chat" pour créer une conversation
- Liste des conversations avec :
  - Icône chat
  - Titre (généré depuis le premier message)
  - Bouton supprimer (visible au hover)
- Responsive : overlay + slide sur mobile

### `components/ChatArea.tsx`
Zone principale d'affichage :
- **Header** : bouton menu (mobile) + logo + titre
- **Welcome screen** : affiché quand pas de messages
- **Messages list** : bulles de conversation
  - Messages utilisateur : alignés à droite
  - Messages assistant : avec avatar Logo à gauche
- **Typing indicator** : 3 points animés pendant le chargement
- Auto-scroll vers le bas à chaque nouveau message

### `components/MessageInput.tsx`
Zone de saisie en bas de l'écran :
- Textarea auto-resize (max 200px)
- Envoi via Enter (Shift+Enter pour nouvelle ligne)
- Bouton envoi (actif seulement si texte non vide)
- Désactivé pendant le chargement
- Disclaimer en dessous

## Flux de données

```
User tape message
       ↓
MessageInput.onSend()
       ↓
useChat.sendMessage()
       ↓
POST /api/chat → Serveur Rust → Ollama
       ↓
Réponse ajoutée aux messages
       ↓
ChatArea re-render avec nouveau message
       ↓
localStorage mis à jour
```

## Build

```bash
cd web
npm install
npm run build    # → génère static/
npm run dev      # → dev server sur :5173
```

Le build produit les fichiers dans `../static/` qui sont servis par le serveur Rust.
