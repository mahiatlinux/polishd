import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

export default function PolishPopup() {
  const [status, setStatus] = useState<string>("processing");

  useEffect(() => {
    const unlisten = listen<string>("status-change", (event) => {
      setStatus(event.payload);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const isError = status === "error";

  return (
    <div className="polish-popup-root">
      <div className={`polish-popup-card${isError ? " error" : ""}`}>
        <div className="polish-popup-dot" />
        <span className="polish-popup-label">
          {isError ? "Failed" : "Polishing\u2026"}
        </span>
      </div>
    </div>
  );
}
