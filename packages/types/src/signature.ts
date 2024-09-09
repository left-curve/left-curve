import type { Hex } from "./encoding";

export type EthPersonalMessage = Hex | string | Uint8Array;

export type Signature = {
  r: Hex;
  s: Hex;
  v: number;
};
