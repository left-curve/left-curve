import { assertDeepEqual } from "@left-curve/utils";

import type { LiveResourceSnapshot } from "./types.js";

export function equalLiveResourceState(previous: LiveResourceSnapshot, next: LiveResourceSnapshot) {
  return previous.status === next.status && previous.error === next.error;
}

export function equalLiveResourcePayload<
  Snapshot extends LiveResourceSnapshot,
  Key extends keyof Snapshot,
>(previous: Snapshot, next: Snapshot, keys: readonly Key[]) {
  if (!equalLiveResourceState(previous, next)) return false;

  for (const key of keys) {
    if (!assertDeepEqual(previous[key], next[key])) return false;
  }

  return true;
}
