import { useCallback, useEffect, useSyncExternalStore } from "react";

// ---------- external store (avoids prop-drilling open state) ----------
const listeners = new Set<() => void>();
let isOpen = false;

function subscribe(cb: () => void): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

function getSnapshot(): boolean {
  return isOpen;
}

function getServerSnapshot(): boolean {
  return false;
}

function setOpen(next: boolean): void {
  if (isOpen === next) return;
  isOpen = next;
  for (const listener of listeners) listener();
}

// ---------- public API ----------

/** Open the search palette imperatively (e.g. from a button press). */
export function openSearch(): void {
  setOpen(true);
}

/** Close the search palette imperatively. */
export function closeSearch(): void {
  setOpen(false);
}

/**
 * Hook that exposes the search palette open/close state and wires up the
 * Cmd+K / Ctrl+K global shortcut. Mount once (e.g. in NovaShell).
 */
export function useSearch() {
  const open = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);

  // Global Cmd+K / Ctrl+K toggle
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen(!isOpen);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const toggle = useCallback(() => setOpen(!isOpen), []);

  return { open, openSearch, closeSearch, toggle } as const;
}
