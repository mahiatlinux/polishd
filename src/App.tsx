import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";

const appWindow = getCurrentWindow();

type Status = "ready" | "processing" | "error" | "no-selection" | "no-editable";
type Theme = "dark" | "light";

function EyeOpen() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
      <circle cx="12" cy="12" r="3"/>
    </svg>
  );
}
function EyeOff() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
      <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
      <line x1="1" y1="1" x2="23" y2="23"/>
    </svg>
  );
}
function CheckIcon() {
  return (
    <svg className="check-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
      <polyline points="20 6 9 17 4 12"/>
    </svg>
  );
}
function SunIcon() {
  return (
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="4"/>
      <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/>
    </svg>
  );
}
function MoonIcon() {
  return (
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
    </svg>
  );
}

export default function App() {
  const [apiKey, setApiKey]     = useState("");
  const [showKey, setShowKey]   = useState(false);
  const [keySaved, setKeySaved] = useState(false);
  const [status, setStatus]     = useState<Status>("ready");
  const [hotkey, setHotkey]     = useState("Ctrl+Shift+E");
  const [transformHotkey, setTransformHotkey] = useState("Ctrl+Shift+D");
  const [recording, setRecording] = useState(false);
  const [hotkeyErr, setHotkeyErr] = useState<string | null>(null);
  const [theme, setTheme] = useState<Theme>("dark");
  const themeHydrated = useRef(false);

  useEffect(() => {
    (async () => {
      try {
        const t = await invoke<string>("get_theme");
        if (t === "light" || t === "dark") {
          setTheme(t);
        } else {
          const ls = localStorage.getItem("polishd-theme");
          if (ls === "light" || ls === "dark") {
            setTheme(ls);
            await invoke("save_theme", { theme: ls }).catch(() => {});
          }
        }
      } catch {
        const ls = localStorage.getItem("polishd-theme");
        if (ls === "light" || ls === "dark") {
          setTheme(ls);
          await invoke("save_theme", { theme: ls }).catch(() => {});
        }
      } finally {
        themeHydrated.current = true;
      }
    })();
  }, []);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("polishd-theme", theme);
    if (!themeHydrated.current) return;
    invoke("save_theme", { theme }).catch(() => {});
  }, [theme]);

  const toggleTheme = () => setTheme((t) => (t === "dark" ? "light" : "dark"));

  useEffect(() => {
    invoke<string>("get_api_key").then((k) => setApiKey(k ?? ""));
    invoke<string>("get_hotkey").then((h: string) => { if (h) setHotkey(h); });
    invoke<string>("get_transform_hotkey").then((h: string) => { if (h) setTransformHotkey(h); });
  }, []);

  useEffect(() => {
    if (!recording) return;
    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      const mods: string[] = [];
      if (e.ctrlKey)  mods.push("Ctrl");
      if (e.altKey)   mods.push("Alt");
      if (e.shiftKey) mods.push("Shift");
      if (e.metaKey)  mods.push("Super");
      const ignore = ["Control", "Alt", "Shift", "Meta"];
      if (ignore.includes(e.key)) return;
      let key = e.key === " " ? "Space"
              : e.key.startsWith("Arrow") ? e.key.replace("Arrow", "")
              : e.key.length === 1 ? e.key.toUpperCase()
              : e.key;
      const combo = [...mods, key].join("+");
      setRecording(false);
      invoke<void>("set_hotkey", { shortcut: combo })
        .then(() => { setHotkey(combo); setHotkeyErr(null); })
        .catch((err: string) => setHotkeyErr(err));
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [recording]);

  useEffect(() => {
    const unlistenStatus = listen<string>("status-change", (e) => {
      setStatus(e.payload as Status);
      if (e.payload === "no-selection" || e.payload === "no-editable") {
        setTimeout(() => setStatus("ready"), 3000);
      }
    });
    return () => {
      unlistenStatus.then((f) => f());
    };
  }, []);

  const handleKeyBlur = useCallback(async () => {
    const ok = await invoke<boolean>("save_api_key", { key: apiKey });
    if (ok) {
      setKeySaved(true);
      setTimeout(() => setKeySaved(false), 2000);
    }
  }, [apiKey]);

  const minimize = () => appWindow.minimize().catch(console.error);
  const closeWin = () => appWindow.hide().catch(console.error);

  const delays = ["80ms", "160ms", "240ms"];

  return (
    <>
      <div className="titlebar" data-tauri-drag-region>
        <div className="titlebar-drag" data-tauri-drag-region />
        <div className="titlebar-btns">
          <button
            className="theme-btn"
            onClick={(e) => { e.stopPropagation(); toggleTheme(); }}
            title={theme === "dark" ? "Switch to light" : "Switch to dark"}
          >
            {theme === "dark" ? <SunIcon /> : <MoonIcon />}
          </button>
          <button className="titlebar-btn" onClick={(e) => { e.stopPropagation(); minimize(); }} title="Minimize">—</button>
          <button className="titlebar-btn" onClick={(e) => { e.stopPropagation(); closeWin(); }} title="Close">✕</button>
        </div>
      </div>

      <div className="win-body">
        <div className="header">
          <div className="header-left">
            <span className="app-title">polishd</span>
          </div>
          <div style={{ display:"flex", flexDirection:"column", alignItems:"flex-end", gap:4 }}>
            <div className="header-right">
              <span className="status-label">
                {status === "processing" ? "Polishing…"
                 : status === "error" ? "Error"
                 : status === "no-selection" ? "No text selected"
                 : status === "no-editable" ? "No editable field"
                 : "Ready"}
              </span>
              <span
                className={`status-dot ${status !== "ready" ? status : ""}`}
                style={
                  status === "no-selection" || status === "no-editable"
                    ? { background: "var(--warn)" }
                    : undefined
                }
              />
            </div>
            <span className="header-sub">Highlight → Hotkey → Polished</span>
          </div>
        </div>

        <div className="card" style={{ animationDelay: delays[0] }}>
          <div className="label">API Key</div>
          <div className="input-row">
            <div className="input-wrap">
              <input
                className="key-input"
                type={showKey ? "text" : "password"}
                placeholder="sk-or-v1-••••••••••••••••••••••"
                value={apiKey}
                onChange={(e) => { setApiKey(e.target.value); setKeySaved(false); }}
                onBlur={handleKeyBlur}
                spellCheck={false}
                autoComplete="off"
              />
              <button className="eye-btn" onClick={() => setShowKey(!showKey)} tabIndex={-1}>
                {showKey ? <EyeOff /> : <EyeOpen />}
              </button>
            </div>
            {keySaved && <CheckIcon />}
          </div>
          <p className="helper">
            OpenRouter key.{" "}
            <a
              href="#"
              onClick={(e) => { e.preventDefault(); openUrl("https://openrouter.ai/keys"); }}
            >
              Get one at openrouter.ai
            </a>
          </p>
        </div>

        <div className="card" style={{ animationDelay: delays[1] }}>
          <div className="label">Shortcut</div>
          <div className="shortcut-row">
            {recording ? (
              <span className="recording-hint">Press your combo…</span>
            ) : (
              <div className="keycaps">
                {hotkey.split("+").map((k, i) => (
                  <span key={i} style={{ display: "contents" }}>
                    {i > 0 && <span className="keycap-plus">+</span>}
                    <span className="keycap">{k}</span>
                  </span>
                ))}
              </div>
            )}
            <button
              className={`btn-record${recording ? " active" : ""}`}
              onClick={() => { setRecording((r) => !r); setHotkeyErr(null); }}
            >
              {recording ? "Cancel" : "Change"}
            </button>
          </div>
          {hotkeyErr && <p className="helper err">{hotkeyErr}</p>}
          <div className="shortcut-alt">
            <span className="shortcut-alt-label">Prompt modal</span>
            <div className="keycaps">
              {transformHotkey.split("+").map((k, i) => (
                <span key={i} style={{ display: "contents" }}>
                  {i > 0 && <span className="keycap-plus">+</span>}
                  <span className="keycap">{k}</span>
                </span>
              ))}
            </div>
          </div>
          <p className="helper">Opens a prompt modal where you can type a custom instruction to run on your selection.</p>
        </div>

        <div className="footer">
          <span>v0.1.0 · Built by Maheswar with ❤️</span>
        </div>
      </div>
    </>
  );
}
