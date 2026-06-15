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

const OPENAI_KEY_STORAGE = "wispr_openai_key";
const ANTHROPIC_KEY_STORAGE = "wispr_anthropic_key";
const GROQ_KEY_STORAGE = "wispr_groq_key";

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
  language: string;
  hotkey: string;
  context_awareness_enabled: boolean;
}

interface ApiKeyInputProps {
  label: string;
  sublabel?: string;
  placeholder: string;
  storageKey: string;
  command: string;
  onSave?: (hasSavedKey: boolean) => void;
}

function ApiKeyInput({ label, sublabel, placeholder, storageKey, command, onSave }: ApiKeyInputProps) {
  const [value, setValue] = useState("");
  const [saved, setSaved] = useState(false);
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const stored = localStorage.getItem(storageKey) ?? "";
    if (stored) {
      setValue(stored);
      setSaved(true);
      invoke(command, { key: stored });
      onSave?.(true);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function save() {
    const key = value.trim();
    localStorage.setItem(storageKey, key);
    invoke(command, { key });
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

  // Settings state — loaded from backend config on mount
  const [cleanupEnabled, setCleanupEnabled] = useState(true);
  const [sttProvider, setSttProvider] = useState<"openai" | "groq">("openai");
  const [language, setLanguage] = useState("en");
  const [hotkey, setHotkey] = useState("ctrl_win");
  const [contextAwareness, setContextAwareness] = useState(true);

  // Derived from localStorage (keys are stored there, pushed to backend on init)
  const [hasAnthropicKey, setHasAnthropicKey] = useState(
    !!(localStorage.getItem(ANTHROPIC_KEY_STORAGE)?.trim())
  );
  const [hasGroqKey, setHasGroqKey] = useState(
    !!(localStorage.getItem(GROQ_KEY_STORAGE)?.trim())
  );

  useEffect(() => {
    invoke<AppSettings>("get_settings").then((s) => {
      setCleanupEnabled(s.cleanup_enabled);
      setSttProvider(s.stt_provider as "openai" | "groq");
      setLanguage(s.language);
      setHotkey(s.hotkey);
      setContextAwareness(s.context_awareness_enabled);
    });

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
  const effectiveCleanup = cleanupEnabled && hasAnthropicKey;

  function handleCleanupToggle() {
    const next = !cleanupEnabled;
    setCleanupEnabled(next);
    invoke("set_cleanup_enabled", { enabled: next });
  }

  function handleProviderChange(provider: "openai" | "groq") {
    setSttProvider(provider);
    invoke("set_stt_provider", { provider });
  }

  function handleLanguageChange(lang: string) {
    setLanguage(lang);
    invoke("set_language", { language: lang });
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
            {effectiveCleanup ? "Claude cleanup: on" : "Claude cleanup: off"}
          </p>

          {errorMsg && <p className="error-msg">{errorMsg}</p>}

          {transcript && recordingState === "idle" && (
            <p className="transcript">&ldquo;{transcript}&rdquo;</p>
          )}

          <div className="keys-section">
            <ApiKeyInput
              label="OpenAI key"
              placeholder="sk-…"
              storageKey={OPENAI_KEY_STORAGE}
              command="set_openai_key"
            />
            <ApiKeyInput
              label="Anthropic key"
              sublabel="(optional — enables cleanup)"
              placeholder="sk-ant-…"
              storageKey={ANTHROPIC_KEY_STORAGE}
              command="set_anthropic_key"
              onSave={setHasAnthropicKey}
            />
            <ApiKeyInput
              label="Groq key"
              sublabel="(optional — ~5× faster transcription)"
              placeholder="gsk_…"
              storageKey={GROQ_KEY_STORAGE}
              command="set_groq_key"
              onSave={setHasGroqKey}
            />
          </div>

          <div className="prefs-section">
            <div className="pref-row">
              <span className="pref-label">Transcription</span>
              <div className="provider-toggle">
                <button
                  className={`provider-btn ${sttProvider === "openai" ? "active" : ""}`}
                  onClick={() => handleProviderChange("openai")}
                >
                  OpenAI
                </button>
                <button
                  className={`provider-btn ${sttProvider === "groq" ? "active" : ""}`}
                  onClick={() => handleProviderChange("groq")}
                  disabled={!hasGroqKey}
                  title={hasGroqKey ? "" : "Add Groq API key above to enable"}
                >
                  Groq
                </button>
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
              <button
                className={`toggle-btn ${cleanupEnabled ? "on" : "off"}`}
                onClick={handleCleanupToggle}
                title={!hasAnthropicKey ? "Set Anthropic key above to activate" : ""}
              >
                {cleanupEnabled ? "On" : "Off"}
              </button>
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
