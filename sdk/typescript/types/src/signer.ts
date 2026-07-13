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
  signArbitrary(payload: ArbitraryDoc): Promise<ArbitrarySignatureOutcome>;
};
