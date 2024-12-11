import type { Credential, StandardCredential } from "./credential.js";
import type { JsonValue } from "./encoding.js";
import type { KeyHash } from "./key.js";
import type { SignDoc } from "./signature.js";

export type Signer = {
  getKeyHash: () => Promise<KeyHash>;
  signArbitrary: (payload: JsonValue) => Promise<StandardCredential>;
  signTx: (signDoc: SignDoc) => Promise<{ credential: Credential; keyHash: KeyHash }>;
};
