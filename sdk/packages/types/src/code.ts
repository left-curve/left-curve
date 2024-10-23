import { Base64 } from "./encoding";

export type Code = {
  readonly code: Base64;
  readonly status: CodeStatus;
}

export type CodeStatus =
  | { orphaned: { since: string } }
  | { inUse: { usage: number } };
