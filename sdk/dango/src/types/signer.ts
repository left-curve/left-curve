import type { Signer as GrugSigner, JsonValue } from "@left-curve/sdk/types";
import type { KeyHash } from "./key.js";
import type {
  ArbitraryDoc,
  ArbitrarySignatureOutcome,
  SignDoc,
  SignatureOutcome,
} from "./signature.js";

export type Signer = GrugSigner<{
  getKeyHash(): Promise<KeyHash>;
  signTx(signDoc: SignDoc): Promise<SignatureOutcome>;
  signArbitrary<T extends JsonValue = JsonValue>(
    payload: ArbitraryDoc<T>,
  ): Promise<ArbitrarySignatureOutcome>;
}>;
