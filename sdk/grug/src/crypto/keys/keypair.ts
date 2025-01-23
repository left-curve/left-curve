export type KeyPair = {
  getPublicKey(compressed?: boolean): Uint8Array;
  createSignature: (hash: Uint8Array) => Uint8Array;
  verifySignature: (hash: Uint8Array, signature: Uint8Array) => boolean;
};
