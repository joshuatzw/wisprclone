# WisprClone

A Wispr Flow-inspired universal voice-to-text desktop app for Windows.

Hold a hotkey, speak naturally, release — cleaned text appears in whatever window has focus.

---

## How it works

```
Hold Ctrl+Win  →  microphone records audio
Release        →  WAV sent to OpenAI Whisper (transcription)
               →  Claude Haiku cleans up the raw transcript
               →  text written to clipboard → Ctrl+V injected into focused app
               →  transcript saved to local history
```

Works in any application that accepts keyboard input: browsers, editors, chat apps, email, terminals.

---

## Stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri v2 |
| Backend | Rust |
| Frontend (settings UI + overlay) | React + TypeScript |
| Audio capture | cpal |
| Speech-to-text | OpenAI Whisper API (`whisper-1`) |
| LLM cleanup | Anthropic Claude Haiku (`claude-haiku-4-5`) |
| Text injection | arboard (clipboard) + enigo (Ctrl+V simulation) |
| Global hotkey | Windows `WH_KEYBOARD_LL` low-level hook |
| System tray | Tauri `tray-icon` feature |

---

## Building

**Prerequisites:** Rust, Node.js, Visual Studio C++ Build Tools

```bash
npm install
npm run tauri build -- --debug   # debug build (~17 MB, fast compile)
npm run tauri build               # release build (optimised)
```

Output: `C:\cargo-targets\wispr-clone\debug\wispr-clone.exe`

For active development with hot reload:
```bash
npm run tauri dev
```

---

## Setup

1. Launch `WisprClone.exe` — the app starts silently in the system tray
2. Left-click the tray icon (or right-click → Settings) to open the settings window
3. Paste your **OpenAI API key** (`sk-…`) into the key field and hit Save
4. Optionally paste your **Anthropic API key** (`sk-ant-…`) to enable the Claude cleanup pass
5. Hold **Ctrl + Win**, speak, release — text appears where your cursor is

Keys are persisted in the app's local storage and pushed to the Rust backend on every launch.

Closing the settings window hides it back to the tray. To quit entirely, right-click the tray icon → Quit WisprClone.

---

## Source layout

```
src/
  App.tsx               Settings window root — tab routing (Settings / History)
  History.tsx           History tab — browse, copy, and delete past transcripts
  Overlay.tsx           Floating recording indicator
  App.css               Shared styles for both windows
src-tauri/src/
  lib.rs                Tauri setup, AppState, tray, hotkey loop, overlay control
  audio.rs              Microphone capture → f32 mono WAV (cpal + hound)
  transcribe.rs         OpenAI Whisper API call
  cleanup.rs            Anthropic Claude Haiku cleanup pass
  inject.rs             Clipboard write + Ctrl+V simulation (arboard + enigo)
  hotkey.rs             Windows low-level keyboard hook (Ctrl+Win push-to-talk)
  history.rs            Transcript persistence — load/save/push to history.json
```

---

## Roadmap

### ✅ Phase 1 — Core recording pipeline
- Global push-to-talk hotkey (Ctrl+Win) via `WH_KEYBOARD_LL` hook
- Microphone capture to f32 mono WAV
- Handles any release order (Win-first or Ctrl-first)

### ✅ Phase 2 — Transcription + text injection
- WAV → OpenAI Whisper API (`whisper-1`, English)
- Transcript written to clipboard → Ctrl+V simulated into focused window

### ✅ Phase 3 — Claude cleanup pass
- Raw Whisper output → Claude Haiku post-processing
- Handles spoken punctuation ("comma", "new line", etc.), filler words, grammar
- Optional: skipped gracefully if no Anthropic key is set

### ✅ Phase 4 — System tray + floating overlay
- App lives in the Windows system tray; no window shown on launch
- Left-click tray icon toggles the settings window; closing it hides to tray
- Tray right-click menu: Settings / Quit WisprClone
- Small transparent always-on-top dot appears center-bottom of screen while recording or processing
- Dot animates through a purple color cycle while recording; turns blue while transcribing/cleaning

### ✅ Phase 5 — History / lookback
- Every dictated transcript saved locally with timestamp
- Settings window gains a History tab — browse and re-copy past dictations
- Entries prepend live as new recordings complete
- Per-entry delete; persisted across restarts in `%APPDATA%\com.joshuatan.wispr-clone\history.json`

### ✅ Phase 6 — Settings UI
- Hotkey customisation: Ctrl+Win (default), Right Alt, Ctrl+Shift, or Ctrl+Alt
- Toggle Claude cleanup on/off independently of the Anthropic key
- Choose STT provider: OpenAI Whisper or Groq (`whisper-large-v3-turbo`, ~5× faster)
- Language selection (13 languages + auto-detect)
- Groq API key input; all preferences persisted to `config.json` in app data dir

### ✅ Phase 7 — Context awareness
- Detects the focused app at the moment the hotkey is pressed (via Windows `GetForegroundWindow` + process name)
- Classifies into five contexts: **Code** (VS Code, Cursor, JetBrains, etc.), **Chat** (Slack, Discord, Teams…), **Email** (Outlook, Thunderbird; Gmail/Outlook in browser), **Terminal** (Windows Terminal, PowerShell, cmd…), **General**
- Each context uses a tailored Claude cleanup prompt: code-safe symbol expansion in editors, casual tone in chat apps, professional prose in email, raw command output in terminals
- Browsers are sub-classified by window title (e.g. a Chrome tab showing "Gmail" maps to Email)
- Togglable via the new **Context** switch in Settings (defaults on)

### 🔜 Phase 8 — Mobile
- React Native app sharing API-call business logic
- Microphone button → same Whisper + Claude pipeline
- iOS and Android

---

## Notes

- The app uses `WH_KEYBOARD_LL` (low-level keyboard hook) rather than `RegisterHotKey` because the Windows key cannot be registered as a non-modifier key through the standard API.
- The Win key press is suppressed by the hook (returns 1) to prevent the Start menu from opening.
- Cargo build output is redirected to `C:\cargo-targets\wispr-clone\` to avoid issues with non-ASCII characters in the OneDrive path.
- The overlay is a second Tauri `WebviewWindow` (label: `overlay`) loaded from the same frontend bundle. `getCurrentWindow().label` at module load time routes it to the `Overlay` component instead of the settings UI.
