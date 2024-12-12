import type { Credential } from "./credential.js";
import type { JsonValue } from "./encoding.js";
import type { KeyHash } from "./key.js";
import type { SignDoc, Signature } from "./signature.js";

export type Signer = {
  getKeyHash: () => Promise<KeyHash>;
  signArbitrary: (payload: JsonValue) => Promise<{ signature: Signature; keyHash: KeyHash }>;
  signTx: (signDoc: SignDoc) => Promise<{ credential: Credential; keyHash: KeyHash }>;
};
