import type { Hex } from "./common";
import type { Credential, Metadata } from "./credential";
import type { Message } from "./tx";

export type Signer = {
  getKeyId: () => Promise<Hex>;
  signTx: (
    msgs: Message[],
    chainId: string,
    sequence: number,
  ) => Promise<{ credential: Credential; data: Metadata }>;
};
