import type { Credential } from "./credential";
import type { KeyHash } from "./key";
import type { Message } from "./tx";

export type Signer = {
  getKeyHash: () => Promise<KeyHash>;
  signTx: (
    msgs: Message[],
    chainId: string,
    sequence: number,
  ) => Promise<{ credential: Credential; keyHash: KeyHash }>;
};
