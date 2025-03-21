/** Forked from https://github.com/wevm/wagmi/blob/main/packages/react/src/hooks/useSyncExternalStoreWithTracked.ts */
import { assertDeepEqual } from "@left-curve/dango/utils";
import { useRef } from "react";
import { useSyncExternalStoreWithSelector } from "use-sync-external-store/shim/with-selector.js";

const isPlainObject = (obj: unknown) => typeof obj === "object" && !Array.isArray(obj);
// biome-ignore lint/complexity/noBannedTypes: Is necessary all functions types
const isFunction = (fn: unknown): fn is Function => typeof fn === "function";

const clearFunctions = (obj: any) => {
  if (isPlainObject(obj)) {
    const newObj = Object.create({});
    for (const [key, value] of Object.entries(obj)) {
      if (isFunction(value)) continue;
      newObj[key] = value;
    }
    return newObj;
  }
  return obj;
};

export function useSyncExternalStoreWithTracked<snapshot extends selection, selection = snapshot>(
  subscribe: (onStoreChange: () => void) => () => void,
  getSnapshot: () => snapshot,
  getServerSnapshot: undefined | null | (() => snapshot) = getSnapshot,
  isEqual: (a: selection, b: selection) => boolean = assertDeepEqual,
) {
  const trackedKeys = useRef<string[]>([]);
  const result = useSyncExternalStoreWithSelector(
    subscribe,
    getSnapshot,
    getServerSnapshot,
    (x) => x,
    (_a_, _b_) => {
      const a = clearFunctions(_a_);
      const b = clearFunctions(_b_);
      if (isPlainObject(a) && isPlainObject(b) && trackedKeys.current.length) {
        for (const key of trackedKeys.current) {
          const valueA = (a as { [_a: string]: any })[key];
          const valueB = (b as { [_b: string]: any })[key];

          const equal = isEqual(valueA, valueB);
          if (!equal) return false;
        }
        return true;
      }
      return isEqual(a, b);
    },
  );

  if (isPlainObject(result)) {
    const trackedResult = { ...result };
    let properties = {};
    for (const [key, value] of Object.entries(trackedResult as { [key: string]: any })) {
      properties = {
        ...properties,
        [key]: {
          configurable: false,
          enumerable: true,
          get: () => {
            if (!trackedKeys.current.includes(key)) {
              trackedKeys.current.push(key);
            }
            return value;
          },
        },
      };
    }
    Object.defineProperties(trackedResult, properties);
    return trackedResult;
  }

  return result;
}
