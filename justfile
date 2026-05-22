set dotenv-load

# ── Dev ──────────────────────────────────────────────────────

dev *ARGS:
    cd engine && cargo tauri dev {{ARGS}}

# ── Build ────────────────────────────────────────────────────

build-gui:
    cd gui && bun run vite build

build-engine *ARGS:
    cd engine && cargo tauri build {{ARGS}}

build: build-gui build-engine

# ── Check ────────────────────────────────────────────────────

typecheck:
    cd gui && bun run vue-tsc --noEmit

check: typecheck
    cd engine && cargo check

# ── Engine (cargo) ──────────────────────────────────────────

clippy:
    cd engine && cargo clippy

fmt:
    cd engine && cargo fmt

fmt-check:
    cd engine && cargo fmt --check
