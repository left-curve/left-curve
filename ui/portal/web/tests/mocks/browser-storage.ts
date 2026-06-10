import { vi } from "vitest";

export function installMockLocalStorage() {
  const storage = new Map<string, string>();

  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      clear: vi.fn(() => storage.clear()),
      getItem: vi.fn((key: string) => storage.get(key) ?? null),
      removeItem: vi.fn((key: string) => storage.delete(key)),
      setItem: vi.fn((key: string, value: string) => storage.set(key, value)),
    },
  });
}
