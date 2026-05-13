import type { JsonValue } from "./encoding.js";
import type { KeyHash } from "./key.js";
import type {
  ArbitraryDoc,
  ArbitrarySignatureOutcome,
  SignDoc,
  SignatureOutcome,
} from "./signature.js";

export type Signer = {
  getKeyHash(): Promise<KeyHash>;
  signTx(signDoc: SignDoc): Promise<SignatureOutcome>;
  signArbitrary<T extends JsonValue = JsonValue>(
    payload: ArbitraryDoc<T>,
  ): Promise<ArbitrarySignatureOutcome>;
};
