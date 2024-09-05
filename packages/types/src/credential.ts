import type { Username } from "./account";
import type { Base64 } from "./common";
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
  | { passkey: Base64 }
  /** An EVM signature signed by a wallet, along with its typedata. */
  | { walletEvm: Base64 };
