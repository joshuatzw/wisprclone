import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CorrectionRule {
  wrong: string;
  correct: string;
}

export function Corrections() {
  const [rules, setRules] = useState<CorrectionRule[]>([]);
  const [wrongInput, setWrongInput] = useState("");
  const [correctInput, setCorrectInput] = useState("");
  const wrongRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    invoke<CorrectionRule[]>("get_corrections").then(setRules).catch(console.error);
  }, []);

  async function addRule() {
    const wrong = wrongInput.trim();
    const correct = correctInput.trim();
    if (!wrong || !correct) return;
    await invoke("add_correction", { wrong, correct });
    setRules((prev) => {
      const key = wrong.toLowerCase();
      const filtered = prev.filter((r) => r.wrong !== key);
      return [...filtered, { wrong: key, correct }].sort((a, b) => a.wrong.localeCompare(b.wrong));
    });
    setWrongInput("");
    setCorrectInput("");
    wrongRef.current?.focus();
  }

  async function removeRule(wrong: string) {
    await invoke("delete_correction", { wrong });
    setRules((prev) => prev.filter((r) => r.wrong !== wrong));
  }

  const canAdd = wrongInput.trim().length > 0 && correctInput.trim().length > 0;

  return (
    <>
      <p className="vocab-description">
        Corrections are applied after transcription. If a word is always wrong,
        add it here and it will be fixed automatically every time.
      </p>

      <div className="corr-add-grid">
        <input
          ref={wrongRef}
          className="vocab-input"
          placeholder="If I say…"
          value={wrongInput}
          onChange={(e) => setWrongInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && canAdd && addRule()}
        />
        <input
          className="vocab-input"
          placeholder="Replace with…"
          value={correctInput}
          onChange={(e) => setCorrectInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && canAdd && addRule()}
        />
        <button className="vocab-add-btn" onClick={addRule} disabled={!canAdd}>
          Add
        </button>
      </div>

      {rules.length === 0 ? (
        <p className="history-empty">
          No corrections yet — add one above or dictate and edit a transcript to teach the app.
        </p>
      ) : (
        <div className="corr-list">
          {rules.map(({ wrong, correct }) => (
            <div key={wrong} className="corr-item">
              <span className="corr-wrong">{wrong}</span>
              <span className="corr-arrow">→</span>
              <span className="corr-correct">{correct}</span>
              <button
                className="history-btn delete"
                onClick={() => removeRule(wrong)}
                aria-label={`Remove correction for ${wrong}`}
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
