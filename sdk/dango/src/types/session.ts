import type { Base64 } from "@left-curve/sdk/types";
import type { StandardCredential } from "./credential.js";
import type { KeyHash } from "./key.js";

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

export type SessionResponse = {
  keyHash: KeyHash;
  authorization: StandardCredential;
  sessionInfo: SigningSessionInfo;
};
