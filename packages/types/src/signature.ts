import type { Hex } from "./encoding";
import type { Message } from "./tx";

export type EthPersonalMessage = Hex | string | Uint8Array;

export type Signature = {
  r: Hex;
  s: Hex;
  v: number;
};

export type SignDoc = {
  msgs: Message[];
  chainId: string;
  sequence: number;
  typedData?: unknown;
};
