import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { LogicalPosition, LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";

const modalWindow = getCurrentWindow();

const MODAL_WIDTH       = 560;
const CARD_PAD_Y        = 11;
const BORDER_Y          = 2;
const LINE_HEIGHT       = 26;
const MIN_CONTENT       = LINE_HEIGHT;
const MAX_CONTENT       = LINE_HEIGHT * 6;
const MIN_WINDOW_HEIGHT = CARD_PAD_Y * 2 + BORDER_Y + MIN_CONTENT;
const MAX_WINDOW_HEIGHT = CARD_PAD_Y * 2 + BORDER_Y + MAX_CONTENT;

declare global {
  interface Window {
    __POLISHD_ANCHOR__?: { x: number; y: number };
  }
}

export default function TransformModal() {
  const [instruction, setInstruction] = useState("");
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
    const rawContent = ta.scrollHeight;
    const clampedContent = Math.max(MIN_CONTENT, Math.min(MAX_CONTENT, rawContent));
    ta.style.height = `${clampedContent}px`;

    const winHeight = Math.max(
      MIN_WINDOW_HEIGHT,
      Math.min(MAX_WINDOW_HEIGHT, CARD_PAD_Y * 2 + BORDER_Y + clampedContent),
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
    const trimmed = instruction.trim();
    if (!trimmed || busy) return;
    setBusy(true);
    try {
      await invoke("submit_transform", { instruction: trimmed });
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
        <svg
          className="transform-icon"
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.25"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <path d="M4 7h7" />
          <path d="M4 12h10" />
          <path d="M4 17h5" />
          <path d="M16 15l4 4" />
          <path d="M18 11l2 2-6 6h-2v-2z" />
        </svg>
        <textarea
          ref={setTextareaRef}
          className="transform-textarea"
          rows={1}
          placeholder="Describe your edit…"
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
        <span className="transform-hint">
          <kbd>↵</kbd>
          <kbd>Esc</kbd>
        </span>
      </div>
    </div>
  );
}
