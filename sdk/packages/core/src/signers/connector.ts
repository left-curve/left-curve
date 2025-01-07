import type {
  ArbitrarySignatureOutcome,
  JsonValue,
  KeyHash,
  SignDoc,
  SignatureOutcome,
  Signer,
} from "@left-curve/types";

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
