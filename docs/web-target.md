# Web Target (wasm32)

The web build is fully working: `dx build --platform web --release` compiles,
bundles (wasm-bindgen) and emits `target/dx/novelcraft/release/web/public` —
the deployable static site. Desktop is
unaffected; storage paths resolve identically on both targets.

## Architecture

| Concern | Desktop | Web |
|---------|---------|-----|
| Storage backend | `tokio::fs` (real files) | **OPFS** via web-sys (`engine/src/util/fs/opfs.rs`) |
| Path roots | platform data/config dirs | logical namespaces `data/NovelCraft/…`, `config/NovelCraft/…` under the OPFS root (`engine/src/paths.rs`) |
| Config boot | async `load()` behind the `booted` gate in `App` (`gui/src/main.rs`) — nothing renders until the engine is initialized |
| Window chrome | custom `TitleBar` + `ResizeHandles` | stubbed to empty rsx (browser provides chrome) — `gui/src/titlebar.rs` |
| LLM transport | reqwest hyper + `bytes_stream` | web-sys `fetch` + `ReadableStream` reader (`kiruklaw-agent-loop` `fetch_stream`, upstream) |

The engine's fs surface is cfg-split in `engine/src/util/fs.rs`
(`ensure_dir`, `remove_file`, `list_dir`, `read`/`write` via `serialize`/
`deserialize`); both impls share the same
signatures, so engine code above it is target-agnostic. OPFS ops treat
missing paths as empty/no-op (listing → `[]`, removal → `Ok`), matching the
native helpers' semantics. Sync OPFS access does not exist on the main
thread, so the config boot is async — uniformly on all targets (the old
`load_sync` path was removed).

## Streaming on wasm

reqwest's `stream` feature is a no-op on wasm (`bytes_stream` does not
exist), so kiruklaw-agent-loop issues the chat-completion request as a raw
browser `fetch` (web-sys `RequestInit`) and unwraps the body's
`ReadableStream` via `ReadableStreamDefaultReader` + `futures` `unfold` —
SSE chunks stream to the GUI exactly as on desktop. Upstream commit:
"add wasm fetch-streaming transport".

## CORS

The web app calls the LLM server (`DEFAULT_HOST` `http://localhost:8888/v1`)
cross-origin. `llama-server` sends `Access-Control-Allow-Origin: *` by
default, so local dev works out of the box; hosted deployments need the
header on whatever serves the model.

## Useful commands

```bash
just build-web                     # release wasm build (just css + dx build --platform web --release)
dx serve --platform web            # hot-reload web dev
just check-gui                     # checks the GUI for desktop AND web (wasm32) targets
```

**Never** run plain `cargo check --target wasm32-unknown-unknown` for the GUI:
the default features compile the desktop renderer, whose websocket/TLS stack
fails on `openssl-sys` (it cannot target wasm at all). The wasm pass must
negate the defaults and enable `dioxus/web` — exactly what `just check-gui`
does.

## Version pinning

The wasm-only JS-interop deps are pinned in lockstep across
`engine/Cargo.toml` and kiruklaw's `agent-loop/Cargo.toml`:
`wasm-bindgen 0.2.129`, `wasm-bindgen-futures 0.4`, `js-sys 0.3.106`,
`web-sys 0.3.106` (wasm-bindgen-futures renumbers to 0.4.x and pins
`wasm-bindgen =0.2.129` exactly). When bumping, bump everywhere.
