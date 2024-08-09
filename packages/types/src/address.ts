import { Sha256 } from "@leftcurve/crypto";
import { decodeHex, encodeHex } from "@leftcurve/encoding";

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
