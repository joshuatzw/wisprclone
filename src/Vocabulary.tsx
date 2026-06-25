import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface VocabWord {
  word: string;
  count: number;
}

export function Vocabulary() {
  const [words, setWords] = useState<VocabWord[]>([]);
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    invoke<VocabWord[]>("get_vocabulary").then(setWords).catch(console.error);
  }, []);

  async function addWord() {
    const word = input.trim();
    if (!word) return;
    await invoke("add_vocab_word", { word });
    setWords((prev) => {
      const existing = prev.find((w) => w.word === word);
      if (existing) {
        return prev.map((w) => w.word === word ? { ...w, count: Math.max(w.count, 2) } : w);
      }
      return [{ word, count: 2 }, ...prev];
    });
    setInput("");
    inputRef.current?.focus();
  }

  async function removeWord(word: string) {
    await invoke("delete_vocab_word", { word });
    setWords((prev) => prev.filter((w) => w.word !== word));
  }

  const active = words.filter((w) => w.count >= 2);
  const learning = words.filter((w) => w.count < 2);

  return (
    <>
      <p className="vocab-description">
        Words here are fed to the transcription and cleanup engines so your custom terms,
        names, and slang are recognized correctly. Words are learned automatically as you
        dictate — or add them manually below.
      </p>

      <div className="vocab-add-row">
        <input
          ref={inputRef}
          className="vocab-input"
          placeholder="Add a word or name…"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && addWord()}
        />
        <button className="vocab-add-btn" onClick={addWord} disabled={!input.trim()}>
          Add
        </button>
      </div>

      {active.length === 0 && learning.length === 0 ? (
        <p className="history-empty">
          No vocabulary yet — dictate a few times and your words will appear here.
        </p>
      ) : (
        <>
          {active.length > 0 && (
            <section className="vocab-section">
              <h3 className="vocab-section-title">Active ({active.length})</h3>
              <div className="vocab-list">
                {active.map(({ word, count }) => (
                  <div key={word} className="vocab-item">
                    <span className="vocab-word">{word}</span>
                    <span className="vocab-count">{count}×</span>
                    <button
                      className="history-btn delete"
                      onClick={() => removeWord(word)}
                      aria-label={`Remove ${word}`}
                    >
                      ✕
                    </button>
                  </div>
                ))}
              </div>
            </section>
          )}

          {learning.length > 0 && (
            <section className="vocab-section">
              <h3 className="vocab-section-title">Learning (seen once)</h3>
              <div className="vocab-list">
                {learning.map(({ word, count }) => (
                  <div key={word} className="vocab-item learning">
                    <span className="vocab-word">{word}</span>
                    <span className="vocab-count">{count}×</span>
                    <button
                      className="history-btn delete"
                      onClick={() => removeWord(word)}
                      aria-label={`Remove ${word}`}
                    >
                      ✕
                    </button>
                  </div>
                ))}
              </div>
            </section>
          )}
        </>
      )}
    </>
  );
}
