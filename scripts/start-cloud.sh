#!/bin/bash
# Script de démarrage pour RunPod (cloud)
# Usage: ./scripts/start-cloud.sh

set -e

echo "=== Halldyll Agent - Cloud Startup ==="

# Vérifier si Ollama est installé
if ! command -v ollama &> /dev/null; then
    echo "[1/4] Installing Ollama..."
    curl -fsSL https://ollama.com/install.sh | sh
else
    echo "[1/4] Ollama already installed"
fi

# Démarrer Ollama en background
echo "[2/4] Starting Ollama server..."
pkill ollama 2>/dev/null || true
nohup ollama serve > /tmp/ollama.log 2>&1 &
sleep 3

# Vérifier si le modèle est présent, sinon le télécharger
MODEL="mistral:7b-instruct-q8_0"
if ! ollama list | grep -q "$MODEL"; then
    echo "[3/4] Downloading model $MODEL..."
    ollama pull $MODEL
else
    echo "[3/4] Model $MODEL already available"
fi

# Démarrer le serveur Halldyll
echo "[4/4] Starting Halldyll Agent server..."
cd "$(dirname "$0")/.."

if [ -f "./target/release/halldyll_agent" ]; then
    exec ./target/release/halldyll_agent
else
    echo "Error: Binary not found. Run 'cargo build --release' first."
    exit 1
fi
