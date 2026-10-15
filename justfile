set dotenv-load

# ── Dev ──────────────────────────────────────────────────────

# Compile the Tailwind stylesheet once (minified).
css:
    cd gui && bunx @tailwindcss/cli -i input.css -o assets/tailwind.css --minify

# Run the app with hot-reload (Tailwind watcher + Dioxus dev server).
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    cd gui
    cleanup() { kill "${TW_PID:-0}" 2>/dev/null || true; }
    trap cleanup EXIT
    bunx @tailwindcss/cli -i input.css -o assets/tailwind.css --watch &
    TW_PID=$!
    dx serve --platform desktop
llama:
  llama serve --port 8888 -hf ggml-org/gemma-4-26B-A4B-it-GGUF:Q4_0

# ── Build ────────────────────────────────────────────────────

# Bundle the app for the current platform (installers under target/dx).
build:
    just css
    cd gui && dx bundle --platform desktop

# ── Check ────────────────────────────────────────────────────

check:
  cargo check

# ── Engine (cargo) ──────────────────────────────────────────

check-engine:
  cargo check -p novelcraft-engine

check-gui:
  cargo check -p novelcraft-gui

clippy:
  cargo clippy

fmt:
  cargo fmt

fmt-check:
  cargo fmt --check
