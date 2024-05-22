import { Sha256 } from "@cosmjs/crypto";
import { type Message, decodeHex, encodeBigEndian32, encodeHex, encodeUtf8, serialize } from ".";

/**
 * Given parameters used while instantiating a new contract, compute what the
 * contract address would be.
 *
 * Mirrors that Rust function: `grug::Addr::compute`.
 *
 * @param deployer Address of the deployer, that it, the account that sends the
 * `Message::Instantiate`.
 * @param codeHash SHA-256 hash of the Wasm byte code.
 * @param salt An arbitrary byte array chosen by the deployer.
 * @returns The address that is given to the contract being instantiated.
 */
export function createAddress(deployer: string, codeHash: Uint8Array, salt: Uint8Array): string {
  const hasher = new Sha256();
  hasher.update(decodeHex(deployer.substring(2))); // strip the 0x prefix
  hasher.update(codeHash);
  hasher.update(salt);
  const bytes = hasher.digest();
  return "0x" + encodeHex(bytes);
}

/**
 * Derive the salt that is used by the standard account factory contract to
 * register accounts.
 *
 * Mirrors the Rust function: `grug_account_factory::make_salt`.
 *
 * @param publicKeyType A string identifying the type of public key. Can either
 * be `secp256k1` or `secp256r1`.
 * @param publicKeyBytes The public key as a byte array.
 * @param serial A serial number. The account to be created under this public
 * key gets serial 0, the second gets 1, so on.
 * @returns The salt that the account factory will use to instantiate the account.
 */
export function createSalt(
  publicKeyType: "secp256k1" | "secp256r1",
  publicKeyBytes: Uint8Array,
  serial: number,
): Uint8Array {
  const hasher = new Sha256();
  hasher.update(encodeUtf8(publicKeyType));
  hasher.update(publicKeyBytes);
  hasher.update(encodeBigEndian32(serial));
  return hasher.digest();
}

/**
 * Generate sign byte that the grug-account contract expects.
 *
 * Mirrors the Rust function: `grug_account::sign_bytes`.
 */
export function createSignBytes(
  msgs: Message[],
  sender: string,
  chainId: string,
  sequence: number,
): Uint8Array {
  const hasher = new Sha256();
  hasher.update(serialize(msgs));
  hasher.update(decodeHex(sender.substring(2))); // strip the 0x prefix
  hasher.update(encodeUtf8(chainId));
  hasher.update(encodeBigEndian32(sequence));
  return hasher.digest();
}
