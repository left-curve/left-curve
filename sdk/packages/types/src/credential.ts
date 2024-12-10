import type { Username } from "./account.js";
import type { Address } from "./address.js";
import type { Base64 } from "./encoding.js";
import type { KeyHash } from "./key.js";
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

export type SessionCredential = {
  /** The `SessionInfo` that contains data to be signed with user key and otp key. */
  sessionInfo: SessionInfo;
  /** Signature of the `SignDoc` by the session key. */
  sessionSignature: Base64;
  /** Signatures of the `SessionInfo` by the user key and OTP. */
  sessionInfoSignature: StandardCredential;
};

export type SessionInfo = {
  /** Public key of the session key. */
  sessionKey: Base64;
  /** Expiry time of the session key. */
  expireAt: number;
  /** Addresses that can use the session key. */
  whitelistedAccounts: Address[];
};
