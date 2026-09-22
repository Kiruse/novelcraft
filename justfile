set dotenv-load

# ── Dev ──────────────────────────────────────────────────────

dev:
  cargo run --bin novelcraft
llama:
  llama serve --port 8888 -hf ggml-org/gemma-4-26B-A4B-it-GGUF:Q4_0

# ── Build ────────────────────────────────────────────────────

build:
  cargo build

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
