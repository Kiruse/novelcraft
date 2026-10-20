# Known Issues

Development-only issues that are not worth fixing in the app itself. Don't
chase these as bugs — check here first before investigating crashes.

## SIGSEGV on paste after hot-reload (dev-only)

**Symptom:** pasting into any text input (e.g. the gameplay prompt area)
segfaults the whole app (SIGSEGV in `libwebkit2gtk` clipboard/pasteboard
handling, UI process). The clipboard may also appear emptied afterwards.

**Trigger:** the clipboard content was copied **before** a `dx` hot-reload.
Hot-reload rebuilds and replaces the app's process image, invalidating the
memory the clipboard data originated from — pasting that stale clipboard
content crashes WebKitGTK when it dereferences the dead buffer.

**Scope:** development only (`dx serve` hot-reload). Release builds never
hot-reload, and freshly copied content is unaffected regardless.

**Workaround:** re-copy after each hot-reload before pasting, or avoid
pasting clipboard content that predates the last rebuild.
