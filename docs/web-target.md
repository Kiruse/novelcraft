# Web Target (wasm32) — Investigation Notes

Findings from the first `dx build --platform web` attempt. The desktop build is
unaffected by everything below; the groundwork is in place so a web port is a
series of small, well-scoped steps rather than a rewrite.

## Current state

`dx build --platform web` compiles the whole dependency graph and fails in
`tokio` on `rt-multi-thread`, which arrives **from kiruklaw-agent-loop's own
Cargo.toml** (unconditioned). Everything novelcraft-owned below that wall is
either already wasm-clean or has a known, tiny fix (see checklist).

The platform feature split works like this:

- `gui/Cargo.toml` declares only `dioxus = { features = ["router"] }` plus
  `default = ["dioxus/desktop"]` (so plain `cargo check` / rust-analyzer build
  the desktop renderer).
- `dx build/serve --platform <p>` negates the default and injects the matching
  `dioxus/<p>` feature. **Never put platform features in the dioxus dependency
  list itself** — with both `desktop` and `web` enabled, dioxus-desktop
  compiles on wasm and drags in tungstenite → native-tls → openssl (which
  cannot build for wasm).

## Groundwork already merged

| Change | Where | Why |
|--------|-------|-----|
| Workspace tokio trimmed to `macros, sync, time, io-util, rt` | root `Cargo.toml` | `fs`/`net`/`process`/`rt-multi-thread` are hard-rejected on wasm ("Only features sync,macros,io-util,rt,time are supported") |
| Per-target tokio `fs` re-add | `engine/Cargo.toml`, `gui/Cargo.toml` (`cfg(not(target_arch = "wasm32"))`) | Desktop keeps `tokio::fs`; wasm never sees it |
| `getrandom` `wasm_js` backend (uuid v4 needs it) | `gui/Cargo.toml` wasm target deps + `.cargo/config.toml` rustflags | `getrandom` ≥0.3 requires the feature AND `--cfg getrandom_backend="wasm_js"` on `wasm32-unknown-unknown` |
| uuid `js` feature (wasm target only) | `engine/Cargo.toml` | uuid's wasm randomness source |

## Remaining checklist (in order)

1. **kiruklaw-agent-loop (upstream)**
   - tokio: gate `rt-multi-thread` behind `cfg(not(target_arch = "wasm32"))`
     (or plain `rt` — the loop runs on the dx-injected runtime anyway).
   - reqwest: the `stream` feature is a no-op on wasm — `bytes_stream` cannot
     work over fetch. SSE streaming needs a fetch/ReadableStream (web-sys)
     transport behind a cfg. This is the biggest upstream item.
2. **Engine storage abstraction** — the *only* engine blockers, all filesystem:
   - `engine/src/util/mod.rs`: `ensure_dir`, `serialize`, `deserialize`,
     `deserialize_sync` (cfg-split native `tokio::fs` vs web backend).
   - `engine/src/paths.rs`: `dirs::data_dir`/`config_dir` — on web these become
     logical namespace keys, not real paths.
   - `engine/src/game/session.rs` `list_sessions`: `read_dir` enumeration →
     backend listing.
   - `engine/src/game/engine.rs` (~l.111–118): sync config load/save.
   Web backend choice: **OPFS or IndexedDB** (sessions are whole-file JSON;
   localStorage's ~5 MB cap is too tight). A `Storage` trait with
   native/web impls behind cfg keeps desktop byte-identical.
3. **GUI cfg-gating** — desktop-only chrome behind `cfg`:
   - `main.rs`: `dioxus::desktop::Config` / window builder (web launch takes no
     window config).
   - `titlebar.rs`: custom window chrome + resize handles (the browser provides
     its own).
4. **CORS** — the web app calls the LLM server (`DEFAULT_HOST`
   `http://localhost:8888/v1`) cross-origin. llama.cpp's `llama-server` sends
   `Access-Control-Allow-Origin: *` by default, so local dev works out of the
   box; hosted deployments need the header on whatever serves the model.

## Useful commands

```bash
dx build --platform web            # full wasm build (fails at kiruklaw today)
dx serve --platform web            # hot-reload web dev
cargo check --target wasm32-unknown-unknown --features dioxus/web --no-default-features
```
