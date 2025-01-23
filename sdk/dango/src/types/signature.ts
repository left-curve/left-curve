import type {
  Base64,
  ArbitrarySignatureOutcome as GrugArbitrarySignatureOutcome,
  SignDoc as GrugSignDoc,
  SignatureOutcome as GrugSignatureOutcome,
} from "@left-curve/types";
import type { Credential } from "./credential.js";
import type { Metadata } from "./metadata.js";

export type SignDoc = GrugSignDoc<Metadata>;

export type SignatureOutcome = GrugSignatureOutcome<Metadata, Credential>;

export type ArbitrarySignatureOutcome = GrugArbitrarySignatureOutcome<Credential>;

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
