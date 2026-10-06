# OmniCoder

Cross-platform AI coding assistant — a chat-first Tauri app with agentic tool use and granular PC access controls.

## Overview

OmniCoder is a Tauri v2 application (Rust + Vue 3/Vite) that provides a conversational agent interface connected to an OmniRoute gateway. It supports:

- **Chat-first UI** (Chat, Code, Terminal, Providers, Plugins tabs)
- **Streaming agent loop** with multi-turn reasoning and tool calling (SSE + OpenAI-style tool calls)
- **PC access permissions**: `readonly`, `ask` (default), or `auto` for tool execution
- **Tool sandboxing** (home/workspace-scoped reads/writes, command allowlist with risk gating)
- **Persistent conversations** (localStorage), cancellation, markdown rendering
- **Default model**: `gemini/gemini-3.1-flash-lite`

## Architecture

- **Frontend**: Vue 3, Vite, Pinia (`chatStore`, `aiStore`), marked + DOMPurify
- **Backend**: Tauri v2 (Rust), commands exposed via `generate_handler!` (`chat`, `chat_cancel`, `exec_tool`, `list_tools`)
- **Agent core**: `src-tauri/src/agent.rs` — streaming, tool registry, sandbox, approvals
- **Gateway**: connects to `http://localhost:20128/v1` (OmniRoute) — configurable via env `OMNIROUTE_BASE_URL`, `OMNIROUTE_API_KEY`

## Requirements

- Linux (Wayland/X11), Tauri v2 prerequisites
- Node.js 20+, pnpm/npm
- Rust (stable) + cargo
- OmniRoute gateway running locally on `:20128` (optional for UI; required for live chat)

## Development

```bash
cd app
npm i --legacy-peer-deps
npm run dev  # Vite dev server (http://localhost:5173)
# or run Tauri dev
npx tauri dev
```

## Build

```bash
cd app
npm run build
npx tauri build --release
```

Linux release bundles are written to `src-tauri/target/release/bundle/` (e.g. `.deb`, `.rpm`, `.AppImage`, `.tar.gz`).

## Usage

1. Launch OmniCoder (`omnicoder` binary or app bundle)
2. Open Chat tab and type a message
3. With `ask` mode (default), write/command tools prompt for approval before execution
4. Tools run sandboxed to `$HOME`/workspace; reads are non-blocking by default

## Permissions

| Mode | Read files/dirs/search | Write files | Run commands |
|---|---|---|---|
| `readonly` | auto | ask/deny | ask/deny |
| `ask` | auto | ask | ask |
| `auto` | auto | auto | auto (risk-gated) |

## Testing

```bash
# Frontend
npm test

# Rust
cd src-tauri
cargo test --release
```

## Documentation

- `docs/` — architecture, agent, tools, streaming, approvals, E2E notes
- `AGENTS.md` — agent instructions and conventions
- `CLAUDE.md` — Claude-specific guidance

## Download

Latest release: [GitHub Releases](https://github.com/fqhd/omnicoder/releases/latest) — download the Linux bundle for your distro (`.deb` recommended for Debian/Ubuntu-based, `.AppImage` portable).

## License

See [LICENSE](LICENSE) if present.


## Quick Download (prebuilt binary)

If no distro-specific bundle was built, download the standalone Linux x64 binary:

- [omnicoder-linux-x64](https://github.com/fqhd/omnicoder/releases/latest/download/omnicoder-linux-x64) (chmod +x)

Run: `chmod +x omnicoder-linux-x64 && ./omnicoder-linux-x64`
