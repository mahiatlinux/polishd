import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const MODAL_WIDTH   = 720;
const LINE_HEIGHT   = 26;
const MIN_CONTENT   = LINE_HEIGHT;
const MAX_CONTENT   = LINE_HEIGHT * 5;
const CHROME_HEIGHT = 81;

type Mode = "transform" | "prompt";

const MODES: Mode[] = ["transform", "prompt"];

const MODE_LABELS: Record<Mode, string> = {
  transform: "Transform",
  prompt: "Prompt",
};

const PLACEHOLDERS: Record<Mode, string> = {
  transform: "Describe your edit…",
  prompt: "Additional context… (optional)",
};

function ModeIcon({ mode }: { mode: Mode }) {
  if (mode === "transform") {
    return (
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
        <path d="m18 14 4 4-4 4"/>
        <path d="m18 2 4 4-4 4"/>
        <path d="M2 18h1.973a4 4 0 0 0 3.3-1.7l5.454-8.6a4 4 0 0 1 3.3-1.7H22"/>
        <path d="M2 6h1.972a4 4 0 0 1 3.6 2.2"/>
      </svg>
    );
  }
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="4 17 10 11 4 5"/>
      <line x1="12" y1="19" x2="20" y2="19"/>
    </svg>
  );
}

declare global {
  interface Window {
    __POLISHD_THEME__?: string;
  }
}

export default function TransformModal() {
  const [instruction, setInstruction] = useState("");
  const [mode, setMode] = useState<Mode>("transform");
  const [busy, setBusy] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const focusInput = useCallback(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    try {
      ta.focus({ preventScroll: true });
      ta.setSelectionRange(ta.value.length, ta.value.length);
    } catch {
      ta.focus();
    }
  }, []);

  const setTextareaRef = useCallback((el: HTMLTextAreaElement | null) => {
    textareaRef.current = el;
    if (!el) return;
    const grab = () => {
      try { window.focus(); } catch {}
      try {
        el.focus({ preventScroll: true });
        el.setSelectionRange(el.value.length, el.value.length);
      } catch {
        el.focus();
      }
    };
    grab();
    requestAnimationFrame(grab);
    requestAnimationFrame(() => requestAnimationFrame(grab));
    window.setTimeout(grab, 60);
    window.setTimeout(grab, 160);
    window.setTimeout(grab, 320);
  }, []);

  const syncSize = useCallback(() => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.style.height = "0px";
    const content = Math.max(MIN_CONTENT, Math.min(MAX_CONTENT, ta.scrollHeight));
    ta.style.height = `${content}px`;
    const height = CHROME_HEIGHT + content;
    invoke("resize_transform", { width: MODAL_WIDTH, height }).catch((err) => {
      console.error("resize_transform failed", err);
    });
  }, []);

  useEffect(() => {
    syncSize();
    focusInput();
    const t1 = window.setTimeout(focusInput, 30);
    const t2 = window.setTimeout(focusInput, 120);
    const onFocus = () => focusInput();
    window.addEventListener("focus", onFocus);
    const onDocKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        invoke("cancel_transform").catch(console.error);
      }
    };
    window.addEventListener("keydown", onDocKeyDown);
    return () => {
      window.clearTimeout(t1);
      window.clearTimeout(t2);
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("keydown", onDocKeyDown);
    };
  }, [focusInput, syncSize]);

  useEffect(() => { syncSize(); }, [instruction, syncSize]);

  const submit = async () => {
    if (busy) return;
    if (mode === "transform" && !instruction.trim()) return;
    setBusy(true);
    try {
      await invoke("submit_transform", { instruction: instruction.trim(), mode });
    } catch (err) {
      console.error("submit_transform failed", err);
      setBusy(false);
    }
  };

  const canSubmit = !busy && (mode === "prompt" || !!instruction.trim());

  return (
    <div className="transform-root">
      <div className={`transform-card${busy ? " busy" : ""}`}>
        <textarea
          ref={setTextareaRef}
          className="transform-textarea"
          rows={1}
          placeholder={PLACEHOLDERS[mode]}
          value={instruction}
          onChange={(e) => setInstruction(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); submit(); }
          }}
          spellCheck={false}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          disabled={busy}
          autoFocus
        />
        <div className="transform-toolbar no-divider">
          <div className="transform-mode-toggle" role="tablist">
            {MODES.map((m) => (
              <button
                key={m}
                className={`transform-mode-toggle-btn${mode === m ? " active" : ""}`}
                onClick={() => { setMode(m); focusInput(); }}
                tabIndex={-1}
                role="tab"
                aria-selected={mode === m}
              >
                <ModeIcon mode={m} />
                <span>{MODE_LABELS[m]}</span>
              </button>
            ))}
          </div>

          <div className="transform-toolbar-right">
            <kbd className="transform-esc-key">Esc</kbd>
            <button
              className="transform-submit"
              onClick={submit}
              disabled={!canSubmit}
              tabIndex={-1}
              aria-label="Submit"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M12 19V5"/>
                <path d="M5 12l7-7 7 7"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}