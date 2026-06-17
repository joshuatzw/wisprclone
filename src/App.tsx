import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Overlay } from "./Overlay";
import { History } from "./History";
import "./App.css";

const WINDOW_LABEL = getCurrentWindow().label;

if (WINDOW_LABEL === "overlay") {
  document.documentElement.classList.add("overlay-window");
}

type RecordingState = "idle" | "recording" | "transcribing" | "cleaning";
type Tab = "settings" | "history";
type SttProvider = "openai" | "groq" | "gemini";
type CleanupSelection = "off" | "anthropic" | "gemini";
type Theme = "light" | "dark" | "auto";

const THEME_STORAGE = "wispr_theme";

function applyTheme(theme: Theme) {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const resolved = theme === "auto" ? (prefersDark ? "dark" : "light") : theme;
  document.documentElement.classList.toggle("light", resolved === "light");
}

// Apply theme before first render to avoid flash
applyTheme((localStorage.getItem(THEME_STORAGE) as Theme) ?? "auto");

const OPENAI_KEY_STORAGE = "wispr_openai_key";
const ANTHROPIC_KEY_STORAGE = "wispr_anthropic_key";
const GROQ_KEY_STORAGE = "wispr_groq_key";
const GEMINI_KEY_STORAGE = "wispr_gemini_key";

const STATUS_LABEL: Record<Exclude<RecordingState, "idle">, string> = {
  recording: "Listening…",
  transcribing: "Transcribing…",
  cleaning: "Cleaning up…",
};

const HOTKEY_LABELS: Record<string, string> = {
  ctrl_win: "Ctrl+Win",
  right_alt: "Right Alt",
  ctrl_shift: "Ctrl+Shift",
  ctrl_alt: "Ctrl+Alt",
};

const LANGUAGES = [
  { value: "auto", label: "Auto-detect" },
  { value: "en", label: "English" },
  { value: "ja", label: "Japanese" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "de", label: "German" },
  { value: "zh", label: "Chinese" },
  { value: "ko", label: "Korean" },
  { value: "pt", label: "Portuguese" },
  { value: "it", label: "Italian" },
  { value: "ru", label: "Russian" },
  { value: "ar", label: "Arabic" },
  { value: "hi", label: "Hindi" },
];

interface AppSettings {
  cleanup_enabled: boolean;
  stt_provider: string;
  cleanup_provider: string;
  language: string;
  hotkey: string;
  context_awareness_enabled: boolean;
  input_device: string;
}

interface AudioDeviceInfo {
  id: string;
  name: string;
  is_default: boolean;
}

interface ApiKeyInputProps {
  label: string;
  sublabel?: string;
  placeholder: string;
  storageKey: string;
  name: string;
  onSave?: (hasSavedKey: boolean) => void;
}

function ApiKeyInput({ label, sublabel, placeholder, storageKey, name, onSave }: ApiKeyInputProps) {
  const [value, setValue] = useState("");
  const [saved, setSaved] = useState(false);
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const stored = localStorage.getItem(storageKey) ?? "";
    if (stored) {
      setValue(stored);
      setSaved(true);
      invoke("set_api_key", { name, key: stored });
      onSave?.(true);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function save() {
    const key = value.trim();
    localStorage.setItem(storageKey, key);
    invoke("set_api_key", { name, key });
    setSaved(true);
    onSave?.(key.length > 0);
    ref.current?.blur();
  }

  return (
    <div className="key-row">
      <label className="key-label">
        {label}
        {sublabel && <span className="key-optional"> {sublabel}</span>}
      </label>
      <div className="key-input-group">
        <input
          ref={ref}
          type="password"
          placeholder={placeholder}
          value={value}
          onChange={(e) => { setValue(e.target.value); setSaved(false); }}
          onKeyDown={(e) => e.key === "Enter" && save()}
          className="key-input"
        />
        <button
          onClick={save}
          className={`key-btn ${saved ? "saved" : ""}`}
          disabled={!value.trim()}
        >
          {saved ? "Saved" : "Save"}
        </button>
      </div>
    </div>
  );
}

function SettingsApp() {
  const [tab, setTab] = useState<Tab>("settings");
  const [recordingState, setRecordingState] = useState<RecordingState>("idle");
  const [transcript, setTranscript] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [sttProvider, setSttProvider] = useState<SttProvider>("openai");
  const [cleanupSelection, setCleanupSelection] = useState<CleanupSelection>("anthropic");
  const [language, setLanguage] = useState("en");
  const [inputDevice, setInputDevice] = useState("");
  const [audioDevices, setAudioDevices] = useState<AudioDeviceInfo[]>([]);
  const [hotkey, setHotkey] = useState("ctrl_win");
  const [contextAwareness, setContextAwareness] = useState(true);
  const [theme, setTheme] = useState<Theme>(
    () => (localStorage.getItem(THEME_STORAGE) as Theme) ?? "auto"
  );

  const [hasAnthropicKey, setHasAnthropicKey] = useState(
    !!(localStorage.getItem(ANTHROPIC_KEY_STORAGE)?.trim())
  );
  const [hasGeminiKey, setHasGeminiKey] = useState(
    !!(localStorage.getItem(GEMINI_KEY_STORAGE)?.trim())
  );

  useEffect(() => {
    applyTheme(theme);
    if (theme !== "auto") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => applyTheme("auto");
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme]);

  useEffect(() => {
    invoke<AppSettings>("get_settings").then((s) => {
      setSttProvider(s.stt_provider as SttProvider);
      setCleanupSelection(
        s.cleanup_enabled ? (s.cleanup_provider as CleanupSelection) : "off"
      );
      setLanguage(s.language);
      setInputDevice(s.input_device);
      setHotkey(s.hotkey);
      setContextAwareness(s.context_awareness_enabled);
    });
    invoke<AudioDeviceInfo[]>("list_audio_devices")
      .then(setAudioDevices)
      .catch((e) => setErrorMsg(String(e)));

    const listeners = [
      listen<string>("recording-state", (e) => {
        setRecordingState(e.payload as RecordingState);
        if (e.payload !== "idle") setErrorMsg(null);
      }),
      listen<string>("transcript", (e) => setTranscript(e.payload)),
      listen<string>("error-message", (e) => setErrorMsg(e.payload)),
    ];
    return () => { listeners.forEach((p) => p.then((f) => f())); };
  }, []);

  const isProcessing = recordingState === "transcribing" || recordingState === "cleaning";

  const cleanupKeyAvailable =
    cleanupSelection === "gemini" ? hasGeminiKey :
    cleanupSelection === "anthropic" ? hasAnthropicKey : false;
  const effectiveCleanup = cleanupSelection !== "off" && cleanupKeyAvailable;

  const cleanupBadgeLabel = effectiveCleanup
    ? `${cleanupSelection === "gemini" ? "Gemini" : "Claude"} cleanup: on`
    : "cleanup: off";

  function handleSttProviderChange(provider: SttProvider) {
    setSttProvider(provider);
    invoke("set_stt_provider", { provider });
  }

  function handleCleanupChange(value: string) {
    setCleanupSelection(value as CleanupSelection);
    if (value === "off") {
      invoke("set_cleanup_enabled", { enabled: false });
    } else {
      invoke("set_cleanup_enabled", { enabled: true });
      invoke("set_cleanup_provider", { provider: value });
    }
  }

  function handleLanguageChange(lang: string) {
    setLanguage(lang);
    invoke("set_language", { language: lang });
  }

  function handleInputDeviceChange(device: string) {
    setInputDevice(device);
    invoke("set_input_device", { device });
  }

  function handleHotkeyChange(hk: string) {
    setHotkey(hk);
    invoke("set_hotkey_combo", { hotkey: hk });
  }

  function handleContextAwarenessToggle() {
    const next = !contextAwareness;
    setContextAwareness(next);
    invoke("set_context_awareness_enabled", { enabled: next });
  }

  function handleThemeChange(t: Theme) {
    setTheme(t);
    localStorage.setItem(THEME_STORAGE, t);
  }

  const selectedDeviceMissing =
    inputDevice !== "" && !audioDevices.some((device) => device.id === inputDevice);

  return (
    <main className="container">
      <h1 className="title">WisprClone</h1>

      <div className="tabs">
        <button
          className={`tab-btn ${tab === "settings" ? "active" : ""}`}
          onClick={() => setTab("settings")}
        >
          Settings
        </button>
        <button
          className={`tab-btn ${tab === "history" ? "active" : ""}`}
          onClick={() => setTab("history")}
        >
          History
        </button>
      </div>

      {tab === "settings" ? (
        <>
          <div className={`mic-ring ${recordingState !== "idle" ? "active" : ""} ${isProcessing ? "processing" : ""}`}>
            {isProcessing ? (
              <span className="spinner" />
            ) : (
              <svg viewBox="0 0 24 24" fill="currentColor" width="32" height="32">
                <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z" />
                <path d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z" />
              </svg>
            )}
          </div>

          <p className="hint">
            {recordingState === "idle"
              ? `Hold ${HOTKEY_LABELS[hotkey] ?? "Ctrl+Win"} to record`
              : STATUS_LABEL[recordingState]}
          </p>

          <p className={`cleanup-badge ${effectiveCleanup ? "on" : "off"}`}>
            {cleanupBadgeLabel}
          </p>

          {errorMsg && <p className="error-msg">{errorMsg}</p>}

          {transcript && recordingState === "idle" && (
            <p className="transcript">&ldquo;{transcript}&rdquo;</p>
          )}

          <div className="keys-section">
            {sttProvider === "openai" && (
              <ApiKeyInput
                label="OpenAI key"
                placeholder="sk-…"
                storageKey={OPENAI_KEY_STORAGE}
                name="openai"
              />
            )}
            {sttProvider === "groq" && (
              <ApiKeyInput
                label="Groq key"
                placeholder="gsk_…"
                storageKey={GROQ_KEY_STORAGE}
                name="groq"
              />
            )}
            {(sttProvider === "gemini" || cleanupSelection === "gemini") && (
              <ApiKeyInput
                label="Gemini key"
                placeholder="AIza…"
                storageKey={GEMINI_KEY_STORAGE}
                name="gemini"
                onSave={setHasGeminiKey}
              />
            )}
            {cleanupSelection === "anthropic" && (
              <ApiKeyInput
                label="Anthropic key"
                placeholder="sk-ant-…"
                storageKey={ANTHROPIC_KEY_STORAGE}
                name="anthropic"
                onSave={setHasAnthropicKey}
              />
            )}
          </div>

          <div className="prefs-section">
            <div className="pref-row">
              <span className="pref-label">Microphone</span>
              <div className="pref-select-wrap">
                <select
                  className="pref-select"
                  value={inputDevice}
                  onChange={(e) => handleInputDeviceChange(e.target.value)}
                >
                  <option value="">System default</option>
                  {selectedDeviceMissing && (
                    <option value={inputDevice}>{inputDevice} (unavailable)</option>
                  )}
                  {audioDevices.map((device) => (
                    <option key={device.id} value={device.id}>
                      {device.name}{device.is_default ? " (default)" : ""}
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="pref-row">
              <span className="pref-label">Transcription</span>
              <div className="pref-select-wrap">
                <select
                  className="pref-select"
                  value={sttProvider}
                  onChange={(e) => handleSttProviderChange(e.target.value as SttProvider)}
                >
                  <option value="openai">OpenAI Transcribe</option>
                  <option value="groq">Groq Whisper</option>
                  <option value="gemini">Gemini</option>
                </select>
              </div>
            </div>

            <div className="pref-row">
              <span className="pref-label">Language</span>
              <div className="pref-select-wrap">
                <select
                  className="pref-select"
                  value={language}
                  onChange={(e) => handleLanguageChange(e.target.value)}
                >
                  {LANGUAGES.map((l) => (
                    <option key={l.value} value={l.value}>{l.label}</option>
                  ))}
                </select>
              </div>
            </div>

            <div className="pref-row">
              <span className="pref-label">Cleanup</span>
              <div className="pref-select-wrap">
                <select
                  className="pref-select"
                  value={cleanupSelection}
                  onChange={(e) => handleCleanupChange(e.target.value)}
                >
                  <option value="off">Off</option>
                  <option value="anthropic">Claude (Anthropic)</option>
                  <option value="gemini">Gemini</option>
                </select>
              </div>
            </div>

            <div className="pref-row">
              <span className="pref-label">Context</span>
              <button
                className={`toggle-btn ${contextAwareness ? "on" : "off"}`}
                onClick={handleContextAwarenessToggle}
                title="Detect the focused app and adjust output style (code, chat, email, terminal)"
              >
                {contextAwareness ? "On" : "Off"}
              </button>
            </div>

            <div className="pref-row">
              <span className="pref-label">Appearance</span>
              <div className="provider-toggle">
                {(["light", "dark", "auto"] as Theme[]).map((t) => (
                  <button
                    key={t}
                    className={`provider-btn ${theme === t ? "active" : ""}`}
                    onClick={() => handleThemeChange(t)}
                  >
                    {t.charAt(0).toUpperCase() + t.slice(1)}
                  </button>
                ))}
              </div>
            </div>

            <div className="pref-row">
              <span className="pref-label">Hotkey</span>
              <div className="pref-select-wrap">
                <select
                  className="pref-select"
                  value={hotkey}
                  onChange={(e) => handleHotkeyChange(e.target.value)}
                >
                  <option value="ctrl_win">Ctrl + Win</option>
                  <option value="right_alt">Right Alt</option>
                  <option value="ctrl_shift">Ctrl + Shift</option>
                  <option value="ctrl_alt">Ctrl + Alt</option>
                </select>
              </div>
            </div>
          </div>
        </>
      ) : (
        <History />
      )}
    </main>
  );
}

function App() {
  if (WINDOW_LABEL === "overlay") return <Overlay />;
  return <SettingsApp />;
}

export default App;
