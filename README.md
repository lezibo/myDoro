最新消息：
  codex已经集成了宠物功能，比我这个好用。我这个没有实时的简略对话与多会话管理，后续会加上；我这个有随机图、眼部追踪鼠标；。可以理解为我的这个实用性差点，外观好点。
使用方式：
  1、先安装 codex宠物相关的skill，直接在codex app 中输入：“$skill-installer hatch-pet”
  2、对话中输入‘/’唤起刚才安装的宠物相关的skill‘hatch-pet’，之后输入“帮我创建一个新宠物 新的形象可以参考${放doro的图片路径}，这次动作仅涉及形象的变更”
  3、等codex 执行完之后 在 设置 -> 设置 -> pets 中最后一行找到自定义宠物就可以了
<p align="center">
  <img src="assets/tray-icon.png" width="128" alt="myDoro">
</p>
<h1 align="center">myDoro</h1>
<p align="center">
  A birthday gift to myself, for loving myself and loving life
  <br>
  <a href="README.zh-CN.md">中文版</a>
</p>
<p align="center">
  <img src="https://img.shields.io/badge/v0.1.0-blue" alt="version">
  <img src="https://img.shields.io/badge/Tauri_v2-orange" alt="Tauri v2">
  <img src="https://img.shields.io/badge/Svelte_5-red" alt="Svelte 5">
  <img src="https://img.shields.io/badge/Rust-black" alt="Rust">
  <img src="https://img.shields.io/badge/Windows%20%7C%20macOS%20%7C%20Linux-grey" alt="platforms">
</p>

myDoro is a birthday gift to myself. It keeps "love yourself, love life" at the center: a small desktop companion that watches my AI coding agents while I work, then occasionally pops out to be playful.

When an agent needs permission during development, myDoro shows a desktop reminder so I can approve or deny it in time. It also reacts to the agent's state: thinking when prompted, typing when tools run, juggling subagents, celebrating completed work, and sleeping quietly when I step away.

Works with **Claude Code**, **Codex CLI**, and **Copilot CLI** — all three can run simultaneously.

## Quick Start

```bash
git clone https://github.com/lezibo/myDoro.git
cd myDoro
npm install
npm start        # Tauri dev mode with hot-reload
```

**Prerequisites** — [Node.js](https://nodejs.org/) v18+, [Rust](https://rustup.rs/) stable, and [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

**Agent setup** — all zero-config:
- **Claude Code** — hooks auto-registered on launch (command hooks + HTTP permission hook)
- **Codex CLI** — log polling starts automatically (`~/.codex/sessions/`)
- **Copilot CLI** — auto-configured when `~/.copilot` exists

## Features

### Animations

12 animated states driven by real-time agent events:

| Agent Event | myDoro Does | Preview |
|---|---|---|
| Idle | Follows your cursor (eye tracking + body lean) | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-idle.png" width="80" /> |
| UserPromptSubmit | Thinking | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-thinking.png" width="80" /> |
| PreToolUse | Typing | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-typing.png" width="80" /> |
| 3+ sessions active | Building | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-building.png" width="80" /> |
| 1 subagent | Juggling | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-juggling.png" width="80" /> |
| 2+ subagents | Conducting | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-conducting.png" width="80" /> |
| PostToolUseFailure | Error flash | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-error.png" width="80" /> |
| Stop (task complete) | Happy bounce | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-happy.png" width="80" /> |
| Notification | Alert jump | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-notification.png" width="80" /> |
| PreCompact | Sweeping | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-sweeping.png" width="80" /> |
| WorktreeCreate | Carrying box | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-carrying.png" width="80" /> |
| 60s no activity | Yawn → doze → collapse → sleep | <img src="https://raw.githubusercontent.com/lezibo/myDoro/main/doroPic/previews/doro-sleeping.png" width="80" /> |

### Interaction

- **Drag** anywhere, anytime — Pointer Capture prevents fast-flick drops
- **Double-click** for a poke reaction; **4 clicks** for a flail
- **Right-click** context menu — session list, DND, mini mode, size, language
- **System tray** — resize (S/M/L), DND, mini mode, language, auto-start, quit

### Mini Mode

Drag Clyde to the left or right screen edge (or right-click "Mini Mode"). Clyde hides behind the edge, peeks out on hover, and shows mini alerts/celebrations while tucked away.

### Permission Bubbles

When Claude Code requests tool permissions, Clyde pops a floating card near the pet — allow, deny, or apply a suggestion rule (e.g. "Always allow Read"). Multiple requests stack upward from the pet. If you answer in the terminal first, the bubble auto-dismisses.

Clyde also tracks Claude's **permission mode** in real time. When the mode changes (e.g. switching to "Accept Edits" via `/permissions`), a brief notification appears near the pet:

| Mode | Meaning |
|------|---------|
| Default | Tool calls require your approval |
| Accept Edits | Edit operations auto-approved, others may still need approval |
| Bypass Permissions | No approval bubbles will appear |
| Plan | No tool execution, planning only |

### Session Intelligence

- **Multi-session priority** — the highest-priority state across all sessions wins
- **Subagent-aware** — 1 subagent = juggling, 2+ = conducting
- **Terminal focus** — right-click a session to jump to its terminal
- **Auto-cleanup** — stale sessions removed after 10 min; working states demoted after 5 min
- **DND mode** — silences all events; toggle via right-click or tray

## Architecture

```
src-tauri/src/           Rust backend
├── lib.rs               App entry + Tauri commands
├── state_machine.rs     Multi-session state tracking + priority
├── http_server.rs       Axum HTTP (POST /state, /permission)
├── hooks.rs             Hook deployment + settings.json registration
├── permission.rs        Permission bubble windows
├── mini.rs              Edge snap, peek, parabolic jump
├── tick.rs              50ms cursor poll (eyes, sleep, peek)
├── tray.rs              System tray menu
├── windows.rs           Window bounds + hit-test math
├── focus.rs             Terminal focus by PID (Win/Mac/Linux)
├── codex_monitor.rs     Codex JSONL log polling
├── prefs.rs             Preferences persistence
└── i18n.rs              English / Chinese strings

src/windows/             Svelte 5 frontend (3 windows)
├── pet/                 SVG renderer
├── hit/                 Invisible click layer
└── bubble/              Permission card

hooks/                   JS hooks (embedded at compile time)
├── clyde-hook.js        Claude Code command hook
├── server-config.js     Port discovery
├── auto-start.js        Auto-launch on SessionStart
├── copilot-hook.js      Copilot CLI hook
└── install.js           Manual hook registration CLI

assets/svg/              35 animation frames
```

## Tech Stack

| Layer | Technology | Why |
|---|---|---|
| **Desktop framework** | [Tauri v2](https://v2.tauri.app/) | ~5 MB bundle (vs 150 MB+ for Electron); native OS APIs (transparent windows, tray, global shortcuts); Rust backend calls with zero IPC serialization overhead |
| **Backend** | [Rust](https://www.rust-lang.org/) | No GC, zero-cost abstractions; 50 ms timer + multi-session state machine in a single process with near-zero CPU; `Mutex` + `Arc` for thread safety by default |
| **Frontend** | [Svelte 5](https://svelte.dev/) | Compile-time, no virtual DOM — three windows total < 30 KB JS; `$state` / `$props` reactivity keeps SVG rendering logic minimal |
| **HTTP server** | [Axum](https://github.com/tokio-rs/axum) | Async web framework on Tokio; type-safe routing + extractors; shares the same Tokio runtime as Tauri — no extra thread pool |
| **Build tool** | [Vite](https://vitejs.dev/) | Instant HMR in dev; aggressive tree-shaking in production |

**Why this stack:** Rust owns all state logic and system interaction, Svelte is a razor-thin rendering layer, and Tauri glues them into a < 10 MB cross-platform desktop app. No runtime interpreter (Node.js, Python, etc.) — cold start < 1 s, resident memory < 30 MB.

## Known Limitations

| Limitation | Details |
|---|---|
| Codex: no terminal focus | JSONL polling doesn't carry terminal PID |
| Copilot: no permission bubble | Copilot's hook protocol only supports deny |
| HTTP server is unauthenticated | Binds `127.0.0.1` only; token auth planned |
| No auto-update | Download new versions from GitHub Releases |

## Troubleshooting

### macOS: "App is damaged and can't be opened"

This is macOS Gatekeeper blocking unsigned apps — the app is not actually damaged. Fix:

```bash
xattr -cr "/Applications/Clyde on Desk.app"
codesign --force --deep --sign - "/Applications/Clyde on Desk.app"
```

The first command clears the quarantine flag, the second adds an ad-hoc signature (required on Apple Silicon).

### Permission bubbles not appearing

If Clyde's permission approval bubbles don't show when Claude Code requests tool permissions:

1. In Claude Code, run `/hooks` and check that `PermissionRequest` has an `[http]` hook
2. If missing or malformed, restart Clyde — it re-registers hooks on startup
3. If still broken, run `node hooks/install.js` manually
4. As a last resort, delete the `PermissionRequest` entry from `~/.claude/settings.json` and restart Clyde

The correct format in `~/.claude/settings.json` should look like:

```json
"PermissionRequest": [
  {
    "matcher": "",
    "hooks": [
      { "type": "http", "url": "http://127.0.0.1:23333/permission", "timeout": 600 }
    ]
  }
]
```

> Permission bubbles only appear for tools that trigger Claude Code's `PermissionRequest` event.

## Origin

myDoro evolved from [QingJ01/Clyde](https://github.com/QingJ01/Clyde). Thanks to the original project for the inspiration, structure, and code foundation that made it possible to turn this into a birthday gift to myself.

## License

[AGPL-3.0](LICENSE)
