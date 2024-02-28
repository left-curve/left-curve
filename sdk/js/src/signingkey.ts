import {
  Bip39,
  EnglishMnemonic,
  Secp256k1,
  type Secp256k1Keypair,
  Sha256,
  Slip10,
  Slip10Curve,
  stringToPath,
} from "@cosmjs/crypto";
import {
  type Addr,
  Binary,
  type Message,
  type Tx,
  encodeBigEndian32,
  encodeUtf8,
  serialize,
} from ".";

/**
 * An secp256k1 private key, with useful methods.
 */
export class SigningKey {
  private keyPair: Secp256k1Keypair;

  /**
   * Do not use; use `fromMnemonic` or `fromFile` instead.
   */
  private constructor(keyPair: Secp256k1Keypair) {
    this.keyPair = keyPair;
  }

  /**
   * Derive an secp256k1 private key pair from the given English mnemonic and
   * BIP-44 coin type.
   */
  public static async fromMnemonic(mnemonic: string, coinType = 60): Promise<SigningKey> {
    const englishMnemonic = new EnglishMnemonic(mnemonic);
    const seed = await Bip39.mnemonicToSeed(englishMnemonic);
    const hdPath = stringToPath(`m/44'/${coinType}'/0'/0/0`);
    const slip10Res = Slip10.derivePath(Slip10Curve.Secp256k1, seed, hdPath);
    const keyPair = await Secp256k1.makeKeypair(slip10Res.privkey);
    return new SigningKey(keyPair);
  }

  /**
   * Create and sign a transaction.
   */
  public async createAndSignTx(
    msgs: Message[],
    sender: Addr,
    chainId: string,
    sequence: number,
  ): Promise<Tx> {
    const signBytes = createSignBytes(msgs, sender, chainId, sequence);
    const extendedSignature = await Secp256k1.createSignature(signBytes, this.keyPair.privkey);
    // important: trim the recovery byte to get the 64-byte signature
    const signature = Secp256k1.trimRecoveryByte(extendedSignature.toFixedLength());
    return {
      sender,
      msgs,
      credential: new Binary(signature),
    };
  }

  public privateKey(): Uint8Array {
    return this.keyPair.privkey;
  }

  public publicKey(): Uint8Array {
    // important: get the compressed 32-byte pubkey instead of the 64-byte one
    return Secp256k1.compressPubkey(this.keyPair.pubkey);
  }
}

/**
 * Generate sign byte that the cw-account contract expects.
 */
export function createSignBytes(
  msgs: Message[],
  sender: Addr,
  chainId: string,
  sequence: number,
): Uint8Array {
  const hasher = new Sha256();
  hasher.update(serialize(msgs));
  hasher.update(sender.bytes);
  hasher.update(encodeUtf8(chainId));
  hasher.update(encodeBigEndian32(sequence));
  return hasher.digest();
}
