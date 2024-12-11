import type {
  JsonValue,
  KeyHash,
  SignDoc,
  SignedDoc,
  Signer,
  StandardCredential,
} from "@left-curve/types";

type Connectorish = {
  getKeyHash(): Promise<KeyHash>;
  signTx(signDoc: SignDoc): Promise<SignedDoc>;
  signArbitrary(data: JsonValue): Promise<{ credential: StandardCredential; keyHash: KeyHash }>;
};

export class ConnectorSigner implements Signer {
  constructor(readonly connector: Connectorish) {}

  async getKeyHash(): Promise<KeyHash> {
    return this.connector.getKeyHash();
  }

  async signTx(signDoc: SignDoc): Promise<SignedDoc> {
    return await this.connector.signTx(signDoc);
  }

  async signArbitrary(
    data: JsonValue,
  ): Promise<{ credential: StandardCredential; keyHash: KeyHash }> {
    return await this.connector.signArbitrary(data);
  }
}
