import type { Username } from "./account.js";
import type { Timestamp } from "./common.js";
import type { KeyHash } from "./key.js";
import type { SessionCredential } from "./session.js";
import type { Signature } from "./signature.js";

export type Metadata = {
  /** The username of the account that signed this transaction */
  username: Username;
  /** Identifies the chain this transaction is intended for. */
  chainId: string;
  /** The nonce this transaction was signed with. */
  nonce: number;
  /** The expiration time of this transaction. */
  expiry?: Timestamp;
};

export type Credential =
  /**Signatures of the authorized key and optional OTP key. */
  | { standard: StandardCredential }
  /** Session credential information with the authorization signatures */
  | { session: SessionCredential };

export type StandardCredential = {
  /** Identifies the key which the user used to sign this transaction. */
  keyHash: KeyHash;
  /** Signature of the `SignDoc` or `SessionInfo` by the user private key. */
  signature: Signature;
};
