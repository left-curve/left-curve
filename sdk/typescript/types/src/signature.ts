import type { Address } from "./address.js";
import type { Base64, JsonValue } from "./encoding.js";
import type { Credential } from "./credential.js";
import type { Key, KeyHash } from "./key.js";
import type { Message } from "./tx.js";

/** Canonical Dango SignDoc — what the user signs to authorize a transaction. */
export type SignDoc = {
  sender: Address;
  /** `bigint` is accepted so values above 2^53 don't lose precision through JS `Number`. */
  gasLimit: number | bigint;
  messages: Message[];
  data: SignDocMetadata;
};

export type SignDocMetadata = {
  chainId: string;
  userIndex: number;
  nonce: number;
  expiry?: string;
};

/** Arbitrary payloads the user signs outside of a transaction context. */
export type ArbitraryDoc = SessionDoc | OnboardDoc;

export type SessionDoc = {
  kind: "session";
  chainId: string;
  sessionKey: string;
  expireAt: string;
};

export type OnboardDoc = {
  kind: "onboard";
  chainId: string;
  key: Key;
  keyHash: KeyHash;
  seed: number;
  referrer?: number;
};

export type SignatureOutcome = {
  credential: Credential;
  signed: SignDoc;
};

export type ArbitrarySignatureOutcome = {
  credential: Credential;
  signed: JsonValue;
};

export type Signature =
  | { secp256k1: Secp256k1Signature }
  | { passkey: PasskeySignature }
  | { eip712: Eip712Signature };

export type Secp256k1Signature = Base64;

export type PasskeySignature = {
  sig: Base64;
  client_data: Base64;
  authenticator_data: Base64;
};

export type Eip712Signature = {
  sig: Base64;
  typed_data: Base64;
};

export type RawSignature = {
  r: string;
  s: string;
  v: number;
};
