import type { Address } from "./address.js";
import type { Credential, Metadata } from "./credential.js";
import type { Base64, Hex } from "./encoding.js";
import type { Message } from "./tx.js";

export type EthPersonalMessage = Hex | string | Uint8Array;

export type RawSignature = {
  r: Hex;
  s: Hex;
  v: number;
};

export type SignDoc = {
  sender: Address;
  messages: Message[];
  gasLimit: number;
  data: Metadata;
};

export type SignatureOutcome = {
  credential: Credential;
  signDoc: SignDoc;
};

export type Signature =
  /** An Secp256k1 signature. */
  | { secp256k1: Secp256k1Signature }
  /** An Secp256r1 signature signed by a Passkey, along with necessary metadata. */
  | { passkey: PasskeySignature }
  /** An EVM signature signed by a wallet, along with its typedata. */
  | { eip712: Eip712Signature };

export type Secp256k1Signature = Base64;

export type PasskeySignature = {
  sig: Base64;
  client_data: Base64;
  authenticator_data: Base64;
};

export type Eip712Signature = {
  sig: Base64;
  /** The EIP712 typed data object containing types, domain and the message. */
  typed_data: Base64;
};
