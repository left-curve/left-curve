import { useStoreWithEqualityFn } from "zustand/traditional";
import { useAppStore } from "../providers/AppProvider";

import type { AppState } from "../providers/AppProvider";

const selectAppState = (state: AppState) => state;

export function useApp(): AppState;
export function useApp<Selection>(
  selector: (state: AppState) => Selection,
  equalityFn?: (previous: Selection, next: Selection) => boolean,
): Selection;
export function useApp<Selection>(
  selector?: (state: AppState) => Selection,
  equalityFn?: (previous: Selection, next: Selection) => boolean,
) {
  const store = useAppStore();
  return useStoreWithEqualityFn(
    store,
    (selector ?? selectAppState) as (state: AppState) => Selection,
    equalityFn,
  );
}
