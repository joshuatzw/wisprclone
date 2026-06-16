import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface HistoryEntry {
  id: number;
  timestamp: number;
  text: string;
}

function formatTime(unix: number): string {
  const diff = Math.floor((Date.now() - unix * 1000) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
  return new Date(unix * 1000).toLocaleDateString();
}

function countWords(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

export function History() {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [copiedId, setCopiedId] = useState<number | null>(null);

  useEffect(() => {
    invoke<HistoryEntry[]>("get_history").then(setEntries).catch(console.error);

    const unlisten = listen<HistoryEntry>("history-entry", (e) => {
      setEntries((prev) => [e.payload, ...prev]);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  async function copy(entry: HistoryEntry) {
    await navigator.clipboard.writeText(entry.text);
    setCopiedId(entry.id);
    setTimeout(() => setCopiedId(null), 1500);
  }

  async function remove(id: number) {
    await invoke("delete_history_entry", { id });
    setEntries((prev) => prev.filter((e) => e.id !== id));
  }

  if (entries.length === 0) {
    return (
      <p className="history-empty">
        No transcripts yet — hold Ctrl+Win to start recording.
      </p>
    );
  }

  const totalWords = entries.reduce((sum, e) => sum + countWords(e.text), 0);

  return (
    <>
      <p className="history-stat">words used: {totalWords.toLocaleString()}</p>
      <div className="history-list">
        {entries.map((entry) => (
          <div key={entry.id} className="history-item">
            <div className="history-meta">
              <span className="history-time">{formatTime(entry.timestamp)}</span>
              <div className="history-actions">
                <button
                  className={`history-btn ${copiedId === entry.id ? "copied" : ""}`}
                  onClick={() => copy(entry)}
                >
                  {copiedId === entry.id ? "Copied!" : "Copy"}
                </button>
                <button
                  className="history-btn delete"
                  onClick={() => remove(entry.id)}
                  aria-label="Delete"
                >
                  ✕
                </button>
              </div>
            </div>
            <p className="history-text">{entry.text}</p>
          </div>
        ))}
      </div>
    </>
  );
}
