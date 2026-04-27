import { ed25519 } from "@noble/curves/ed25519";
import { HDKey } from "@scure/bip32";
import { mnemonicToSeedSync } from "@scure/bip39";

import type { KeyPair } from "./keypair.js";

/**
 * Verify a ed25519 signature
 * @param messageHash - The hash of the message that was signed.
 * @param signature - The signature to verify.
 * @param publicKey - The public key to verify the signature with.
 * @returns True if the signature is valid, false otherwise.
 */
export function ed25519VerifySignature(
  messageHash: Uint8Array,
  signature: Uint8Array,
  publicKey: Uint8Array,
): boolean {
  if (messageHash.length !== 32) {
    throw new Error(`Message hash length must not exceed 32 bytes: ${messageHash.length}`);
  }
  return ed25519.verify(signature, messageHash, publicKey);
}

export class Ed25519 implements KeyPair {
  #privateKey: Uint8Array;
  /**
   * Generate a new ed25519 key pair.
   * @returns A new ed25519 key pair.
   */
  static makeKeyPair(): Ed25519 {
    const privateKey = ed25519.utils.randomPrivateKey();
    return new Ed25519(privateKey);
  }

  /**
   * Derive a ed25519 key pair from a mnemonic.
   * @param mnemonic - The English mnemonic to derive the key pair from.
   * @param coinType - The BIP-44 coin type to derive the key pair for.
   * @returns A new ed25519 key pair.
   */
  static fromMnemonic(mnemonic: string, coinType = 60): Ed25519 {
    const masterSeed = mnemonicToSeedSync(mnemonic);
    const hdKey = HDKey.fromMasterSeed(masterSeed);
    const { privateKey } = hdKey.derive(`m/44'/${coinType}'/0'/0/0`);
    if (!privateKey) throw new Error("Failed to derive private key from mnemonic");
    return new Ed25519(privateKey);
  }

  constructor(privateKey: Uint8Array) {
    if (privateKey.length !== 32) {
      throw new Error(`Private key length must be 32 bytes: ${privateKey.length}`);
    }
    this.#privateKey = privateKey;
  }

  getPublicKey(): Uint8Array {
    return ed25519.getPublicKey(this.#privateKey);
  }

  get privateKey() {
    return this.#privateKey;
  }

  /**
   * Sign a message hash with the private key.
   * @param messageHash - The hash of the message to sign.
   * @returns The signature of the message hash.
   */
  createSignature(messageHash: Uint8Array): Uint8Array {
    if (messageHash.length === 0) {
      throw new Error("Message hash must not be empty");
    }
    if (messageHash.length > 32) {
      throw new Error(`Mesage hash length must not exceed 32 bytes: ${messageHash.length}`);
    }

    return ed25519.sign(messageHash, this.#privateKey);
  }

  /**
   * Verify a signature of a message hash.
   * @param messageHash - The hash of the message that was signed.
   * @param signature - The signature to verify.
   * @returns True if the signature is valid, false otherwise.
   */
  verifySignature(messageHash: Uint8Array, signature: Uint8Array): boolean {
    return ed25519VerifySignature(messageHash, signature, this.getPublicKey());
  }
}
