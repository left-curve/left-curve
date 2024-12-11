import type { Username } from "./account.js";
import type { KeyHash } from "./key.js";
import type { SessionCredential } from "./session.js";
import type { OtpSignature, Signature } from "./signature.js";

export type Metadata = {
  /** Identifies which key was used to signed this transaction. */
  keyHash: KeyHash;
  /** The sequence number this transaction was signed with. */
  sequence: number;
  /** The username of the account that signed this transaction */
  username: Username;
};

export type Credential =
  /**Signatures of the authorized key and optional OTP key. */
  | { standard: StandardCredential }
  /** Session credential information with the authorization signatures */
  | { session: SessionCredential };

export type StandardCredential = {
  /** Signature of a user */
  signature: Signature;
  /** Signature of the OTP key */
  otpSignature?: OtpSignature;
};
