import type { JsonValue } from "./encoding.js";
import type { KeyHash } from "./key.js";
import type { ArbitrarySignatureOutcome, SignDoc, SignatureOutcome } from "./signature.js";

export type Signer = {
  getKeyHash: () => Promise<KeyHash>;
  signArbitrary: (payload: JsonValue) => Promise<ArbitrarySignatureOutcome>;
  signTx: (signDoc: SignDoc, extra: unknown) => Promise<SignatureOutcome>;
};
