import type { StandardCredential } from "./credential.js";
import type { Base64 } from "./encoding.js";

export type SessionCredential = {
  /** The `SigningSessionInfo` that contains data to be signed with user key and otp key. */
  sessionInfo: SigningSessionInfo;
  /** Signature of the `SignDoc` by the session key. */
  sessionSignature: Base64;
  /** Signatures of the `SigningSessionInfo` by the user key */
  authorization: StandardCredential;
};

export type SigningSessionInfo = {
  /** Public key of the session key. */
  sessionKey: Base64;
  /** Expiry time of the session key. */
  expireAt: string;
};

export type SigningSession = {
  publicKey: Uint8Array;
  privateKey: Uint8Array;
  keyHash: string;
  sessionInfo: SigningSessionInfo;
  authorization: StandardCredential;
};
