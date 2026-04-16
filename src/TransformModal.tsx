import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LogicalPosition, LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";

const modalWindow = getCurrentWindow();

const MODAL_WIDTH       = 560;
const CARD_PAD_Y        = 12;
const BORDER_Y          = 2;
const LINE_HEIGHT       = 22;
const TOOLBAR_BLOCK     = 47;
const MENU_HEIGHT       = 90;
const MIN_CONTENT       = LINE_HEIGHT;
const MAX_CONTENT       = LINE_HEIGHT * 5;
const MIN_WINDOW_HEIGHT = CARD_PAD_Y * 2 + BORDER_Y + MIN_CONTENT + TOOLBAR_BLOCK;
const MAX_WINDOW_HEIGHT = CARD_PAD_Y * 2 + BORDER_Y + MAX_CONTENT + TOOLBAR_BLOCK;

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
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
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
    __POLISHD_ANCHOR__?: { x: number; y: number };
    __POLISHD_THEME__?: string;
  }
}

export default function TransformModal() {
  const [instruction, setInstruction] = useState("");
  const [mode, setMode] = useState<Mode>("transform");
  const [menuOpen, setMenuOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const menuOpenRef = useRef(false);
  const anchorRef = useRef({
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

  const applySize = useCallback(async (withMenu: boolean) => {
    const ta = textareaRef.current;
    if (!ta) return;
    ta.style.height = "0px";
    const content = Math.max(MIN_CONTENT, Math.min(MAX_CONTENT, ta.scrollHeight));
    ta.style.height = `${content}px`;
    const base = Math.max(
      MIN_WINDOW_HEIGHT,
      Math.min(MAX_WINDOW_HEIGHT, CARD_PAD_Y * 2 + BORDER_Y + content + TOOLBAR_BLOCK),
    );
    const h = base + (withMenu ? MENU_HEIGHT : 0);
    try {
      await modalWindow.setSize(new LogicalSize(MODAL_WIDTH, h));
      await modalWindow.setPosition(
        new LogicalPosition(anchorRef.current.x, anchorRef.current.top),
      );
    } catch (err) {
      console.error("applySize failed", err);
    }
  }, []);

  const resizeWindow = useCallback(() => applySize(menuOpenRef.current), [applySize]);

  const openMenu = useCallback(() => {
    menuOpenRef.current = true;
    setMenuOpen(true);
    applySize(true);
  }, [applySize]);

  const closeMenu = useCallback(() => {
    menuOpenRef.current = false;
    setMenuOpen(false);
    applySize(false);
  }, [applySize]);

  const selectMode = useCallback((m: Mode) => {
    setMode(m);
    closeMenu();
    focusInput();
  }, [closeMenu, focusInput]);

  useEffect(() => {
    resizeWindow();
    focusInput();
    const t1 = window.setTimeout(focusInput, 30);
    const t2 = window.setTimeout(focusInput, 120);
    const onFocus = () => focusInput();
    window.addEventListener("focus", onFocus);
    const onDocKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        if (menuOpenRef.current) {
          closeMenu();
        } else {
          invoke("cancel_transform").catch(console.error);
        }
      }
    };
    window.addEventListener("keydown", onDocKeyDown);
    return () => {
      window.clearTimeout(t1);
      window.clearTimeout(t2);
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("keydown", onDocKeyDown);
    };
  }, [focusInput, resizeWindow, closeMenu]);

  useEffect(() => { resizeWindow(); }, [instruction, resizeWindow]);

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
        <div className="transform-toolbar">
          <div className="transform-mode-wrap">
            <button
              className={`transform-mode-trigger${menuOpen ? " open" : ""}`}
              onClick={() => (menuOpen ? closeMenu() : openMenu())}
              tabIndex={-1}
            >
              <ModeIcon mode={mode} />
              <span>{MODE_LABELS[mode]}</span>
              <svg
                className="transform-chevron"
                width="10" height="10" viewBox="0 0 24 24"
                fill="none" stroke="currentColor"
                strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"
              >
                <path d="M6 9l6 6 6-6"/>
              </svg>
            </button>

            {menuOpen && (
              <div className="transform-mode-menu">
                {MODES.map((m) => (
                  <button
                    key={m}
                    className={`transform-mode-item${mode === m ? " active" : ""}`}
                    onClick={() => selectMode(m)}
                    tabIndex={-1}
                  >
                    <ModeIcon mode={m} />
                    {MODE_LABELS[m]}
                  </button>
                ))}
              </div>
            )}
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
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
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
