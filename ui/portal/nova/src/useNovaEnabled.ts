import { useSyncExternalStore } from "react";

const STORAGE_KEY = "nova-ui-enabled";

function getSnapshot(): boolean {
  return localStorage.getItem(STORAGE_KEY) !== "false";
}

function getServerSnapshot(): boolean {
  return false;
}

const listeners = new Set<() => void>();

function subscribe(callback: () => void): () => void {
  listeners.add(callback);
  return () => listeners.delete(callback);
}

function notify() {
  listeners.forEach((l) => l());
}

export function setNovaEnabled(enabled: boolean) {
  localStorage.setItem(STORAGE_KEY, String(enabled));
  notify();
  window.location.reload();
}

export function isNovaEnabled(): boolean {
  return getSnapshot();
}

export function useNovaEnabled(): { enabled: boolean; toggle: () => void } {
  const enabled = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
  return {
    enabled,
    toggle: () => setNovaEnabled(!enabled),
  };
}
