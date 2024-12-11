import type { Address } from "./address.js";
import type { StandardCredential } from "./credential.js";
import type { Base64 } from "./encoding.js";

export type SessionCredential = {
  /** The `SigningSessionInfo` that contains data to be signed with user key and otp key. */
  sessionInfo: SigningSessionInfo;
  /** Signature of the `SignDoc` by the session key. */
  sessionSignature: Base64;
  /** Signatures of the `SigningSessionInfo` by the user key and OTP. */
  sessionInfoSignature: StandardCredential;
};

export type SigningSessionInfo = {
  /** Public key of the session key. */
  sessionKey: Base64;
  /** Expiry time of the session key. */
  expireAt: string;
  /** Addresses that can use the session key. */
  whitelistedAccounts: Address[];
};

export type SigningSession = {
  publicKey: Uint8Array;
  privateKey: Uint8Array;
  keyHash: string;
  sessionInfo: SigningSessionInfo;
  sessionInfoSignature: StandardCredential;
};
