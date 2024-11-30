import type { KeyHash, SignDoc, SignedDoc, Signer } from "@left-curve/types";

type Connectorish = {
  getKeyHash(): Promise<KeyHash>;
  requestSignature(signDoc: SignDoc): Promise<SignedDoc>;
};

export class ConnectorSigner implements Signer {
  constructor(readonly connector: Connectorish) {}

  async getKeyHash(): Promise<KeyHash> {
    return this.connector.getKeyHash();
  }

  async signTx(signDoc: SignDoc): Promise<SignedDoc> {
    return await this.connector.requestSignature(signDoc);
  }
}
