import type { JsonValue, KeyHash, SignDoc, Signature, SignedDoc, Signer } from "@left-curve/types";

type Connectorish = {
  getKeyHash(): Promise<KeyHash>;
  signTx(signDoc: SignDoc): Promise<SignedDoc>;
  signArbitrary(data: JsonValue): Promise<{ signature: Signature; keyHash: KeyHash }>;
};

export class ConnectorSigner implements Signer {
  constructor(readonly connector: Connectorish) {}

  async getKeyHash(): Promise<KeyHash> {
    return this.connector.getKeyHash();
  }

  async signTx(signDoc: SignDoc): Promise<SignedDoc> {
    return await this.connector.signTx(signDoc);
  }

  async signArbitrary(data: JsonValue): Promise<{ signature: Signature; keyHash: KeyHash }> {
    return await this.connector.signArbitrary(data);
  }
}
