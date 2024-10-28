import type { Base64 } from "./encoding.js";

export type Code = {
  readonly code: Base64;
  readonly status: CodeStatus;
};

export type CodeStatus = { orphaned: { since: string } } | { inUse: { usage: number } };
