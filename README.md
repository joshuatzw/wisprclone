# WisprClone

A push-to-talk voice-to-text desktop app for Windows. Hold a hotkey, speak naturally, release — your words are transcribed and typed directly into whatever window is focused.

---

## Features

- Push-to-talk recording with a configurable hotkey
- Speech-to-text via **OpenAI Whisper**, **Groq Whisper**, or **Gemini**
- Optional AI cleanup pass that removes filler words, fixes punctuation, and adapts output style to the focused app (code editor, chat, email, terminal)
- Automatic context detection — detects VS Code, Slack, Outlook, terminals, and more
- Audio ducking — lowers all system audio by 90% while you speak
- Whisper boost — automatically amplifies quiet or whispered recordings before transcription
- Transcription history with total word count
- Light / Dark / Auto theme

---

## Setup

### 1. Install

Download the latest release from the [Releases](https://github.com/joshuatzw/wisprclone/releases) page and run the installer.

### 2. Open Settings

Launch WisprClone — it starts silently in the system tray. Click the tray icon to open the Settings window.

### 3. Add your API key

You need **at least one STT (speech-to-text) API key**. Enter it in Settings and click **Save**.

### 4. Start dictating

Click into any text field, hold your hotkey, speak, and release. The transcribed text is typed directly into the focused window.

---

## API Keys

### Transcription — required (choose one)

You must set up exactly one STT provider. Keys for providers you don't use are not needed.

| Provider | Key looks like | Where to get it | Notes |
|---|---|---|---|
| **OpenAI Whisper** *(default)* | `sk-…` | [platform.openai.com/api-keys](https://platform.openai.com/api-keys) | Pay-per-use; most accurate |
| **Groq Whisper** | `gsk_…` | [console.groq.com/keys](https://console.groq.com/keys) | Free tier; ~5× faster than OpenAI |
| **Gemini** | `AIza…` | [aistudio.google.com/apikey](https://aistudio.google.com/apikey) | Free tier available |

> Go to **Settings → Transcription** to select your provider, then enter its key.

### Cleanup — optional

The cleanup pass is an optional second AI step that polishes the raw transcript. If no cleanup key is set, or if cleanup is set to **Off**, the raw transcription is used as-is.

| Provider | Key looks like | Where to get it | Notes |
|---|---|---|---|
| **Claude (Anthropic)** | `sk-ant-…` | [console.anthropic.com/settings/keys](https://console.anthropic.com/settings/keys) | Uses claude-haiku-4-5 |
| **Gemini** | `AIza…` | Same as Gemini STT key — enter once | No extra cost if already using Gemini STT |

> Go to **Settings → Cleanup** and select a provider. The corresponding key field will appear.

**Tip:** If you use Gemini for transcription, you can enable Gemini cleanup at no extra setup — the same key covers both.

---

## Settings Reference

| Setting | Options | Notes |
|---|---|---|
| **Transcription** | OpenAI Whisper / Groq Whisper / Gemini | Which STT API to use |
| **Language** | Auto-detect, English, Japanese, Spanish, French, German, Chinese, Korean, Portuguese, Italian, Russian, Arabic, Hindi | Improves accuracy for non-English speech |
| **Cleanup** | Off / Claude (Anthropic) / Gemini | AI pass to polish the transcript; optional |
| **Context** | On / Off | Detects the focused app and adapts cleanup style; defaults on |
| **Appearance** | Light / Dark / Auto | Auto follows your OS dark mode setting |
| **Hotkey** | Ctrl+Win / Right Alt / Ctrl+Shift / Ctrl+Alt | Key combination to hold while speaking |

---

## Hotkey Notes

| Combo | Behaviour |
|---|---|
| **Ctrl+Win** *(default)* | Win key is suppressed while recording — the Start menu will not open |
| **Right Alt** | Works as a standalone hold key; AltGr (Right Alt + Left Ctrl) is automatically excluded for European keyboard users |
| **Ctrl+Shift** | Either key released stops recording |
| **Ctrl+Alt** | Either key released stops recording |

---

## How It Works

```
Hold hotkey   →  microphone starts recording
               →  system audio ducked 90% so you can hear yourself
Release       →  audio restored
               →  quiet recordings boosted to improve accuracy
               →  audio sent to your chosen STT API
               →  (optional) transcript cleaned up by AI
               →  text typed directly into the focused window
               →  transcript saved to local history
```

---

## Building from Source

**Prerequisites:** [Rust](https://rustup.rs/), [Node.js 18+](https://nodejs.org/), [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

```bash
npm install
npm run tauri dev      # development with hot reload
npm run tauri build    # production build
```

---

## Project Structure

```
src/
  App.tsx          Settings window — tab routing, provider selectors, key inputs
  History.tsx      History tab — browse, copy, and delete past transcripts
  Overlay.tsx      Floating recording indicator (always-on-top dot)
  App.css          Shared styles and CSS custom property theming

src-tauri/src/
  lib.rs           Tauri setup, AppState, tray, hotkey loop, overlay control
  audio.rs         Microphone capture → f32 mono WAV (cpal + hound)
  transcribe.rs    STT dispatch — OpenAI / Groq Whisper, Gemini
  cleanup.rs       AI cleanup dispatch — Anthropic Claude, Gemini
  context.rs       Focused-app detection and context classification
  hotkey.rs        Windows low-level keyboard hook (WH_KEYBOARD_LL)
  normalize.rs     Peak normalisation — boosts quiet/whispered recordings
  volume.rs        WASAPI audio ducking — mutes system audio during recording
  history.rs       Transcript persistence (history.json in app data dir)
  inject.rs        Clipboard write + Ctrl+V simulation
  config.rs        App config persistence (config.json in app data dir)
```

---

## License

MIT
