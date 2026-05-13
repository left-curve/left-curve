export type KeyPair = {
  getPublicKey(compressed?: boolean): Uint8Array;
  createSignature: (hash: Uint8Array, recoveryId?: boolean) => Uint8Array;
  verifySignature: (hash: Uint8Array, signature: Uint8Array) => boolean;
};
