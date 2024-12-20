import type { JsonValue } from "./encoding.js";
import type { KeyHash } from "./key.js";
import type { SignDoc, Signature, SignatureOutcome } from "./signature.js";

export type Signer = {
  getKeyHash: () => Promise<KeyHash>;
  signArbitrary: (payload: JsonValue) => Promise<{ signature: Signature; keyHash: KeyHash }>;
  signTx: (signDoc: SignDoc, extra: unknown) => Promise<SignatureOutcome>;
};
