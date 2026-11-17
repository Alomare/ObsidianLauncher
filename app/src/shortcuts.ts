// Ctrl+A, Ctrl+C and Ctrl+V for the lists of the Mods and Saves tabs. None of them fires while a
// box takes text, or while a dialog or a menu is open: there the keys keep their usual meaning.
// Copy and paste go through the copy and paste events, which carry the clipboard without asking
// the webview for clipboard access, and a copy leaves text selected on the page to the browser.
import { useEffect, useRef } from "react";

/** Whether keys pressed in `el` belong to typing: inputs, text areas, selects and editable text. */
export function typingIn(el: EventTarget | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  if (el.isContentEditable || el.closest("[contenteditable]:not([contenteditable='false'])")) return true;
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement;
}

/** Whether a dialog or a menu is open over the page. */
export function overlayOpen(): boolean {
  return !!document.querySelector("[role='dialog'], [role='alertdialog'], [aria-modal='true'], [role='menu']");
}

/** Whether a list shortcut may act on this event. */
export function listKeysFree(target: EventTarget | null): boolean {
  return !typingIn(target) && !typingIn(document.activeElement) && !overlayOpen();
}

/** Text selected on the page, which a copy keeps to itself. */
const pageSelection = () => {
  const s = window.getSelection?.();
  return !!s && !s.isCollapsed && s.toString().trim() !== "";
};

export type ListKeys = {
  /** Ctrl+A. */
  selectAll?: () => void;
  /** Ctrl+C: the text to copy, or null to leave the copy alone. */
  copy?: () => string | null;
  /** Ctrl+V, with the text on the clipboard. */
  paste?: (text: string) => void;
};

/** Listens for the list shortcuts while `active` (the tab shows). The handlers are read when a key comes. */
export function useListKeys(active: boolean, keys: ListKeys) {
  const ref = useRef(keys);
  ref.current = keys;
  useEffect(() => {
    if (!active) return;
    const down = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey || e.isComposing || e.key.toLowerCase() !== "a") return;
      if (!ref.current.selectAll || !listKeysFree(e.target)) return;
      e.preventDefault();
      ref.current.selectAll();
    };
    const copy = (e: ClipboardEvent) => {
      if (!ref.current.copy || !listKeysFree(e.target) || pageSelection()) return;
      const text = ref.current.copy();
      if (text === null || !e.clipboardData) return;
      e.preventDefault();
      e.clipboardData.setData("text/plain", text);
    };
    const paste = (e: ClipboardEvent) => {
      if (!ref.current.paste || !listKeysFree(e.target)) return;
      const text = e.clipboardData?.getData("text/plain") ?? "";
      if (!text.trim()) return;
      e.preventDefault();
      ref.current.paste(text);
    };
    window.addEventListener("keydown", down);
    document.addEventListener("copy", copy);
    document.addEventListener("paste", paste);
    return () => {
      window.removeEventListener("keydown", down);
      document.removeEventListener("copy", copy);
      document.removeEventListener("paste", paste);
    };
  }, [active]);
}
