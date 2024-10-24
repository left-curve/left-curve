import type { Username } from "./account";
import type { Base64, Hex, Json } from "./encoding";
import type { KeyHash } from "./key";

export type Metadata = {
  /** Identifies which key was used to signed this transaction. */
  keyHash: KeyHash;
  /** The sequence number this transaction was signed with. */
  sequence: number;
  /** The username of the account that signed this transaction */
  username: Username;
};

export type Credential =
  /** An Secp256k1 signature. */
  | { secp256k1: Base64 }
  /** An Ed25519 signature. */
  | { ed25519: Base64 }
  /** An Secp256r1 signature signed by a Passkey, along with necessary metadata. */
  | { passkey: PasskeyCredential }
  /** An EVM signature signed by a wallet, along with its typedata. */
  | { eip712: Eip712Credential };

export type PasskeyCredential = {
  sig: Base64;
  client_data: Base64;
  authenticator_data: Base64;
};

export type Eip712Credential = {
  sig: Base64;
  /** The EIP712 typed data object containing types, domain and the message. */
  typed_data: Base64;
};
