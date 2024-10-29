import type { Credential } from "./credential.js";
import type { KeyHash } from "./key.js";
import type { SignDoc } from "./signature.js";

export type Signer = {
  getKeyHash: () => Promise<KeyHash>;
  signTx: (signDoc: SignDoc) => Promise<{ credential: Credential; keyHash: KeyHash }>;
};
