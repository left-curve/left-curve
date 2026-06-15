import { m } from "@left-curve/foundation/paraglide/messages.js";

type MessageInputs = Record<string, unknown>;

function ensureParaglideStorage() {
  if (typeof globalThis.localStorage?.getItem === "function") return;

  const values = new Map<string, string>();

  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      clear: () => values.clear(),
      getItem: (key: string) => values.get(key) ?? null,
      key: (index: number) => Array.from(values.keys())[index] ?? null,
      get length() {
        return values.size;
      },
      removeItem: (key: string) => {
        values.delete(key);
      },
      setItem: (key: string, value: string) => {
        values.set(key, value);
      },
    },
  });
}

export function message(key: keyof typeof m, inputs?: MessageInputs) {
  ensureParaglideStorage();
  return (m[key] as (inputs?: MessageInputs) => string)(inputs);
}
