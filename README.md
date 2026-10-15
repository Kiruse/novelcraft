# NovelCraft

An offline-first, single-user desktop application for LLM-driven interactive fiction with deterministic gameplay modules and human-authored event system. The AI's purpose is not to create the story, but to tell it.

NovelCraft will eventually support a proprietary sharing platform that allows human authors to share their worlds (story templates) with other players and drives discovery via content curation. However, this platform will be entirely optional. The app remains fully usable offline, including importing & exporting story templates directly.

NovelCraft is a Rust/Cargo workspace split into two crates:

- **[novelcraft-engine](engine/)** — pure Rust library crate containing all business logic: LLM proxy (OpenAI-compatible streaming via `reqwest`), filesystem persistence (JSON files for sessions, stories, lore, profiles), the game agent loop, and file import/export. No UI framework dependency.
- **[novelcraft-gui](gui/)** — Rust binary crate providing the UI, built on [Dioxus](https://dioxuslabs.com/), a React-style Rust framework rendered in the platform's system WebView and styled with Tailwind CSS. Desktop is the primary target; Android/iOS are supported compile targets.

There is no server, no database, and no authentication — all data lives as loose JSON files on disk.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable — pinned via `rust-toolchain.toml`)
- [just](https://github.com/casey/just) (build orchestration)
- [bun](https://bun.sh/) (provisions the Tailwind CSS toolchain in `gui/`)
- The [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started) (`dx`) — used by `just dev` / `just build`
- Linux desktop only: WebKitGTK 4.1 and libxdo (e.g. `sudo pacman -S webkit2gtk-4.1 libxdo`)

## Building & Running

All commands are managed by the root `justfile`. Run `just` with no arguments to list available recipes.

```bash
just dev       # Run the app with hot-reload (Tailwind watcher + dx serve --platform desktop)
just build     # Bundle the app for the current platform (just css + dx bundle --platform desktop)
just css       # Compile the Tailwind stylesheet once (minified)
just check     # Cargo check (entire workspace)
just clippy    # Cargo clippy
```

## Documentation

See the [`docs/`](docs/) directory for comprehensive documentation on architecture, data persistence, the game agent loop, and GUI conventions.

# License
All files in this repository are licensed under GNU GPL 3.0. See [LICENSE](./LICENSE).
