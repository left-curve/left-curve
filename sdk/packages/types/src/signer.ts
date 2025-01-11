import type { Json, JsonValue } from "./encoding.js";
import type { ArbitrarySignatureOutcome, SignDoc, SignatureOutcome } from "./signature.js";

export type Signer<Metadata = Json, Credential = Json> = {
  signArbitrary: (payload: JsonValue) => Promise<ArbitrarySignatureOutcome<Credential>>;
  signTx: (
    signDoc: SignDoc<Metadata>,
    extra: unknown,
  ) => Promise<SignatureOutcome<Metadata, Credential>>;
};
