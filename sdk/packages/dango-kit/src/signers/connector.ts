import type { JsonValue } from "@left-curve/types";

import type { KeyHash } from "../types/key.js";
import type { ArbitrarySignatureOutcome, SignDoc, SignatureOutcome } from "../types/signature.js";
import type { Signer } from "../types/signer.js";

type Connectorish = {
  getKeyHash(): Promise<KeyHash>;
  signTx(signDoc: SignDoc): Promise<SignatureOutcome>;
  signArbitrary(data: JsonValue): Promise<ArbitrarySignatureOutcome>;
};

export class ConnectorSigner implements Signer {
  constructor(readonly connector: Connectorish) {}

  async getKeyHash(): Promise<KeyHash> {
    return this.connector.getKeyHash();
  }

  async signTx(signDoc: SignDoc): Promise<SignatureOutcome> {
    return await this.connector.signTx(signDoc);
  }

  async signArbitrary(data: JsonValue): Promise<ArbitrarySignatureOutcome> {
    return await this.connector.signArbitrary(data);
  }
}
