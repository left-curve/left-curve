import type { Json, JsonValue } from "./encoding.js";
import type { ArbitrarySignatureOutcome, SignDoc, SignatureOutcome } from "./signature.js";

export type Signer<
  Metadata extends Json | undefined = Json | undefined,
  Credential extends Json | undefined = Json | undefined,
> = {
  signArbitrary: (payload: JsonValue) => Promise<ArbitrarySignatureOutcome<Credential>>;
  signTx: (
    signDoc: SignDoc<Metadata>,
    extra: unknown,
  ) => Promise<SignatureOutcome<Metadata, Credential>>;
};
