import { encodeHex, hexToBigInt, isHex } from "@leftcurve/encoding";
import { secp256k1 } from "@noble/curves/secp256k1";
import { HDKey } from "@scure/bip32";
import { mnemonicToSeedSync } from "@scure/bip39";

import type { Hex, Signature } from "@leftcurve/types";
import type { KeyPair } from "./keypair";

/**
 * Recover the public key from a message hash and signature.
 * @param hash - The hash of the message that was signed.
 * @param signature - The signature to recover the public key from.
 * @param compressed - Whether to return the public key in compressed form.
 * @returns The public key that signed the message hash.
 */
export async function recoverPublicKey(
  hash: Hex | Uint8Array,
  _signature_: Hex | Uint8Array | Signature,
  compressed = false,
): Promise<Uint8Array> {
  const hashHex = isHex(hash) ? hash.replace("0x", "") : encodeHex(hash);
  const { secp256k1 } = await import("@noble/curves/secp256k1");

  const signature = (() => {
    if (typeof _signature_ === "object" && "r" in _signature_ && "s" in _signature_) {
      const { r, s, v } = _signature_;
      const recoveryBit = toRecoveryBit(v);
      return new secp256k1.Signature(hexToBigInt(r), hexToBigInt(s)).addRecoveryBit(recoveryBit);
    }
    const signatureHex = isHex(_signature_)
      ? _signature_.replace("0x", "")
      : encodeHex(_signature_);

    const v = Number.parseInt(signatureHex.substring(128), 16);
    const recoveryBit = toRecoveryBit(v);

    return secp256k1.Signature.fromCompact(signatureHex.substring(0, 128)).addRecoveryBit(
      recoveryBit,
    );
  })();

  return signature.recoverPublicKey(hashHex).toRawBytes(compressed);
}

function toRecoveryBit(v: number) {
  if (v === 0 || v === 1) return v;
  if (v === 27) return 0;
  if (v === 28) return 1;
  throw new Error("Invalid recovery value");
}

export class Secp256k1 implements KeyPair {
  #privateKey: Uint8Array;
  /**
   * Generate a new secp256k1 key pair.
   * @param optinalPrivateKey - Optional private key to use.
   *  If not provided, a random private key will be generated.
   * @returns A new secp256k1 key pair.
   */
  static makeKeyPair(optinalPrivateKey?: Uint8Array): Secp256k1 {
    const privateKey = optinalPrivateKey ?? secp256k1.utils.randomPrivateKey();
    return new Secp256k1(privateKey);
  }

  /**
   * Verify a secp256k1 signature
   * @param messageHash - The hash of the message that was signed.
   * @param signature - The signature to verify.
   * @param publicKey - The public key to verify the signature with.
   * @returns True if the signature is valid, false otherwise.
   */
  static verifySignature(
    messageHash: Uint8Array,
    signature: Uint8Array,
    publicKey: Uint8Array,
  ): boolean {
    if (messageHash.length !== 32) {
      throw new Error(`Message hash length must not exceed 32 bytes: ${messageHash.length}`);
    }
    return secp256k1.verify(secp256k1.Signature.fromCompact(signature), messageHash, publicKey);
  }

  /**
   * Derive a secp256k1 key pair from a mnemonic.
   * @param mnemonic - The English mnemonic to derive the key pair from.
   * @param coinType - The BIP-44 coin type to derive the key pair for.
   * @returns A new secp256k1 key pair.
   */
  static fromMnemonic(mnemonic: string, coinType = 60): Secp256k1 {
    const masterSeed = mnemonicToSeedSync(mnemonic);
    const hdKey = HDKey.fromMasterSeed(masterSeed);
    const { privateKey } = hdKey.derive(`m/44'/${coinType}'/0'/0/0`);
    if (!privateKey) throw new Error("Failed to derive private key from mnemonic");
    return new Secp256k1(privateKey);
  }

  /**
   * Compress or uncompress a secp256k1 public key.
   * @param pubKey - The public key to compress or uncompress.
   * @param compress - True to compress the public key, false to uncompress it.
   * @returns The compressed or uncompressed public key.
   */
  static compressPubKey(pubKey: Uint8Array, compress: boolean): Uint8Array {
    if (compress && pubKey.length === 33) return pubKey;
    if (!compress && pubKey.length === 65) return pubKey;
    return secp256k1.ProjectivePoint.fromHex(pubKey).toRawBytes(compress);
  }

  constructor(privateKey: Uint8Array) {
    if (privateKey.length !== 32) {
      throw new Error(`Private key length must be 32 bytes: ${privateKey.length}`);
    }
    this.#privateKey = privateKey;
  }

  get publicKey() {
    return secp256k1.getPublicKey(this.#privateKey);
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

    const signature = secp256k1.sign(messageHash, this.#privateKey, { lowS: true });

    return signature.toCompactRawBytes();
  }

  /**
   * Verify a signature of a message hash.
   * @param messageHash - The hash of the message that was signed.
   * @param signature - The signature to verify.
   * @returns True if the signature is valid, false otherwise.
   */
  verifySignature(messageHash: Uint8Array, signature: Uint8Array): boolean {
    return Secp256k1.verifySignature(messageHash, signature, this.publicKey);
  }
}
