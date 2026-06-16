import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

type OverlayState = "idle" | "recording" | "transcribing" | "cleaning";

export function Overlay() {
  const [state, setState] = useState<OverlayState>("idle");

  useEffect(() => {
    const unlisten = listen<string>("recording-state", (e) => {
      setState(e.payload as OverlayState);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return (
    <div className="overlay-root">
      <div className={`overlay-pill ${state}`} />
    </div>
  );
}
