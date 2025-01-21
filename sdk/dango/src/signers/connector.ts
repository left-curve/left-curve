import type { JsonValue } from "@left-curve/types";

import type {
  ArbitrarySignatureOutcome,
  KeyHash,
  SignDoc,
  SignatureOutcome,
  Signer,
} from "../types/index.js";

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
