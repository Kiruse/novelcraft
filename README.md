# NovelCraft

An offline-first, single-user desktop application for LLM-driven interactive fiction with deterministic gameplay modules and human-authored event system. The AI's purpose is not to create the story, but to tell it.

NovelCraft will eventually support a proprietary sharing platform that allows human authors to share their worlds (story templates) with other players and drives discovery via content curation. However, this platform will be entirely optional. The app remains fully usable offline, including importing & exporting story templates directly.

NovelCraft is a Rust/Cargo workspace split into two crates:

- **[novelcraft-engine](engine/)** — pure Rust library crate containing all business logic: LLM proxy (OpenAI-compatible streaming via `reqwest`), filesystem persistence (JSON files for sessions, stories, lore, profiles), the game agent loop, and file import/export. No UI framework dependency.
- **[novelcraft-gui](gui/)** — Rust binary crate providing the UI, built on [Dioxus](https://dioxuslabs.com/), a React-style Rust framework rendered in the platform's system WebView and styled with Tailwind CSS. Desktop is the primary target; the browser (wasm32, storage via OPFS) is a supported build target — see [Web Target](docs/web-target.md).

There is no server, no database, and no authentication — all data lives as loose JSON files on disk.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable — pinned via `rust-toolchain.toml`)
- [just](https://github.com/casey/just) (build orchestration)
- [bun](https://bun.sh/) (provisions the Tailwind CSS toolchain in `gui/`)
- The [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started) (`dx`) — used by `just dev` / `just build`
- Web builds only: the `wasm32-unknown-unknown` target (`rustup target add wasm32-unknown-unknown`)
- Linux desktop only: WebKitGTK 4.1 and libxdo (e.g. `sudo pacman -S webkit2gtk-4.1 libxdo`)

## Building & Running

All commands are managed by the root `justfile`. Run `just` with no arguments to list available recipes.

```bash
just dev        # Run the app with hot-reload (Tailwind watcher + dx serve --platform desktop)
just build      # Bundle the app for the current platform (just css + dx bundle --platform desktop)
just build-web  # Build the web (wasm32) release bundle for deployment (just css + dx build --platform web --release)
just css        # Compile the Tailwind stylesheet once (minified)
just check      # Cargo check (entire workspace)
just check-gui  # Cargo check the GUI for desktop AND web (wasm32) targets
just clippy     # Cargo clippy
```

> **Note on wasm checks:** never invoke plain `cargo check --target wasm32-unknown-unknown`
> for the GUI — the default features compile the *desktop* renderer, which drags in a
> native TLS/websocket stack and fails on `openssl-sys`. Use `just check-gui` (which
> negates the defaults and enables `dioxus/web` for the wasm pass) or
> `dx build --platform web`.

## Releases

Tag pushes (`v*`) and manual dispatches trigger the
[bundle workflow](.github/workflows/bundle.yml), which builds installers for
Windows (NSIS/MSI) and Linux (AppImage/deb) on GitHub-hosted runners.
macOS is deliberately skipped for now — signing & notarization require a
paid Apple developer account; Apple users are served by the web deployment.

## Web Deployment

Pushes to `main` (and manual dispatches) trigger the
[deploy workflow](.github/workflows/deploy-web.yml), which builds the web
(wasm32) release bundle and publishes it to Cloudflare Pages. One-time
setup:

```bash
bunx wrangler pages project create novelcraft   # or via the CF dashboard
```

Then set two repository secrets: `CLOUDFLARE_API_TOKEN` (with
"Cloudflare Pages — Edit" permission) and `CLOUDFLARE_ACCOUNT_ID`.
The app must be hosted at the domain root (Pages projects are by default);
see [docs/web-target.md](docs/web-target.md) for details.

## Documentation

See the [`docs/`](docs/) directory for comprehensive documentation on architecture, data persistence, the game agent loop, and GUI conventions.

# License
All files in this repository are licensed under GNU GPL 3.0. See [LICENSE](./LICENSE).
