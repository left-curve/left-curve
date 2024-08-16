import { Sha256, ripemd160 } from "@leftcurve/crypto";
import { decodeHex, encodeHex } from "@leftcurve/encoding";
import type { Address } from "@leftcurve/types";

export type PredictAddressParameters = {
  deployer: Address;
  codeHash: Uint8Array;
  salt: Uint8Array;
};

export type PredictAddressReturnType = Address;

/**
 * Given parameters used while instantiating a new contract, compute what the
 * contract address would be.
 *
 * Mirrors that Rust function: `grug::Addr::compute`.
 * @param parameters
 * @param parameters.deployer Address of the deployer, that it,
 * the account that sends the `Message::Instantiate`.
 * @param parameters.codeHash SHA-256 hash of the Wasm byte code.
 * @param parameters.salt An arbitrary byte array chosen by the deployer.
 * @returns The address that is given to the contract being instantiated.
 */
export function predictAddress(parameters: PredictAddressParameters): PredictAddressReturnType {
  const { deployer, codeHash, salt } = parameters;
  const hasher = new Sha256();

  const bytes = hasher
    .update(decodeHex(deployer.substring(2))) // strip the 0x prefix
    .update(codeHash)
    .update(salt)
    .digest();

  return `0x${encodeHex(ripemd160(bytes))}`;
}
