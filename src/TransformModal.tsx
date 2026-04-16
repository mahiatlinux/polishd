import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LogicalPosition, LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";

const modalWindow = getCurrentWindow();

const MODAL_WIDTH       = 560;
const CARD_PAD_Y        = 12;
const BORDER_Y          = 2;
const LINE_HEIGHT       = 22;
const TOOLBAR_BLOCK     = 47;
const MIN_CONTENT       = LINE_HEIGHT;
const MAX_CONTENT       = LINE_HEIGHT * 5;
const MIN_WINDOW_HEIGHT = CARD_PAD_Y * 2 + BORDER_Y + MIN_CONTENT + TOOLBAR_BLOCK;
const MAX_WINDOW_HEIGHT = CARD_PAD_Y * 2 + BORDER_Y + MAX_CONTENT + TOOLBAR_BLOCK;

type Mode = "transform" | "prompt";

const PLACEHOLDERS: Record<Mode, string> = {
  transform: "Describe your edit…",
  prompt:    "Additional context… (optional)",
};

declare global {
  interface Window {
    __POLISHD_ANCHOR__?: { x: number; y: number };
    __POLISHD_THEME__?: string;
  }
}

export default function TransformModal() {
  const [instruction, setInstruction] = useState("");
  const [mode, setMode] = useState<Mode>("transform");
  const [busy, setBusy] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const anchorRef = useRef<{ x: number; top: number }>({
    x: window.__POLISHD_ANCHOR__?.x ?? 0,
    top: window.__POLISHD_ANCHOR__?.y ?? 0,
  });

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

  const resizeWindow = useCallback(async () => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.style.height = "0px";
    const clampedContent = Math.max(MIN_CONTENT, Math.min(MAX_CONTENT, ta.scrollHeight));
    ta.style.height = `${clampedContent}px`;
    const winHeight = Math.max(
      MIN_WINDOW_HEIGHT,
      Math.min(MAX_WINDOW_HEIGHT, CARD_PAD_Y * 2 + BORDER_Y + clampedContent + TOOLBAR_BLOCK),
    );
    try {
      await modalWindow.setSize(new LogicalSize(MODAL_WIDTH, winHeight));
      const anchor = anchorRef.current;
      await modalWindow.setPosition(new LogicalPosition(anchor.x, anchor.top));
    } catch (err) {
      console.error("resizeWindow failed", err);
    }
  }, []);

  useEffect(() => {
    resizeWindow();
    focusInput();
    const t1 = window.setTimeout(focusInput, 30);
    const t2 = window.setTimeout(focusInput, 120);
    const onWindowFocus = () => focusInput();
    window.addEventListener("focus", onWindowFocus);
    const onDocKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        invoke("cancel_transform").catch((err) => {
          console.error("cancel_transform failed", err);
        });
      }
    };
    window.addEventListener("keydown", onDocKeyDown);
    return () => {
      window.clearTimeout(t1);
      window.clearTimeout(t2);
      window.removeEventListener("focus", onWindowFocus);
      window.removeEventListener("keydown", onDocKeyDown);
    };
  }, [focusInput, resizeWindow]);

  useEffect(() => {
    resizeWindow();
  }, [instruction, resizeWindow]);

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

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  };

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
          onKeyDown={onKeyDown}
          spellCheck={false}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          disabled={busy}
          autoFocus
        />
        <div className="transform-toolbar">
          <div className="transform-modes">
            <button
              className={`transform-mode-chip${mode === "transform" ? " active" : ""}`}
              onClick={() => { setMode("transform"); focusInput(); }}
              tabIndex={-1}
            >
              Transform
            </button>
            <button
              className={`transform-mode-chip${mode === "prompt" ? " active" : ""}`}
              onClick={() => { setMode("prompt"); focusInput(); }}
              tabIndex={-1}
            >
              Prompt
            </button>
          </div>
          <span className="transform-hint">
            <kbd>↵</kbd>
            <kbd>Esc</kbd>
          </span>
        </div>
      </div>
    </div>
  );
}
