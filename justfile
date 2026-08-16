set dotenv-load

# ── Dev ──────────────────────────────────────────────────────

dev:
  cargo run --bin novelcraft

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
