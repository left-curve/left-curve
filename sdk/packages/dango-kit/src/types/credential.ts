import type { Base64 } from "@left-curve/types";
import type { KeyHash } from "./key.js";
import type { SigningSessionInfo } from "./session.js";
import type { Signature } from "./signature.js";

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

export type SessionCredential = {
  /** The `SigningSessionInfo` that contains data to be signed with user key and otp key. */
  sessionInfo: SigningSessionInfo;
  /** Signature of the `SignDoc` by the session key. */
  sessionSignature: Base64;
  /** Signatures of the `SigningSessionInfo` by the user key */
  authorization: StandardCredential;
};
