import type { Credential } from "./credential";
import type { KeyHash } from "./key";
import type { SignDoc } from "./signature";

export type Signer = {
  getKeyHash: () => Promise<KeyHash>;
  signTx: (signDoc: SignDoc) => Promise<{ credential: Credential; keyHash: KeyHash }>;
};
