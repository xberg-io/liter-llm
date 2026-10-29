#!/usr/bin/env bash
#MISE description="Start Ollama via Docker Compose for local LLM testing"
set -euo pipefail

compose_file=docker/docker-compose.local-llm.yml
docker compose -f "$compose_file" up -d
echo "Waiting for Ollama health check..."
for i in $(seq 1 120); do
  if docker compose -f "$compose_file" ps --format json | grep -q '"healthy"'; then
    echo "Ollama is ready"
    break
  fi
  if [ "$i" -eq 120 ]; then
    echo "ERROR: Ollama failed to become healthy within 120s"
    exit 1
  fi
  sleep 1
done
