import { useCallback, useSyncExternalStore } from "react";

type ThemeMode = "light" | "dark";

const STORAGE_KEY = "nova-theme-mode";

function getSnapshot(): ThemeMode {
  const attr = document.documentElement.getAttribute("data-mode");
  if (attr === "dark" || attr === "light") return attr;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function getServerSnapshot(): ThemeMode {
  return "light";
}

const listeners = new Set<() => void>();

function subscribe(callback: () => void): () => void {
  listeners.add(callback);
  return () => listeners.delete(callback);
}

function setMode(mode: ThemeMode): void {
  document.documentElement.setAttribute("data-mode", mode);
  localStorage.setItem(STORAGE_KEY, mode);
  for (const listener of listeners) listener();
}

function initTheme(): void {
  const stored = localStorage.getItem(STORAGE_KEY) as ThemeMode | null;
  if (stored === "dark" || stored === "light") {
    document.documentElement.setAttribute("data-mode", stored);
  }
}

initTheme();

export function useNovaTheme() {
  const mode = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);

  const toggle = useCallback(() => {
    setMode(mode === "light" ? "dark" : "light");
  }, [mode]);

  return { mode, toggle } as const;
}
