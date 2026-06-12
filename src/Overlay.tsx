import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

type ActiveState = "recording" | "transcribing" | "cleaning";

export function Overlay() {
  const [state, setState] = useState<ActiveState>("recording");

  useEffect(() => {
    const unlisten = listen<string>("recording-state", (e) => {
      if (e.payload !== "idle") setState(e.payload as ActiveState);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return (
    <div className="overlay-root">
      <div className="overlay-pill">
        <span className={`overlay-dot ${state}`} />
      </div>
    </div>
  );
}
