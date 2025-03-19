import type {
  Base64,
  SignDoc as GrugSignDoc,
  SignatureOutcome as GrugSignatureOutcome,
  JsonValue,
} from "@left-curve/sdk/types";
import type { Credential } from "./credential.js";

import type { ArbitraryTypedData, TxMessageType, TypedData } from "./typedData.js";

export type SignDoc = GrugSignDoc<TypedData<TxMessageType>>;

export type ArbitraryDoc<T extends JsonValue = JsonValue> = ArbitraryTypedData<T>;

export type SignatureOutcome = GrugSignatureOutcome<SignDoc, Credential>;

export type ArbitrarySignatureOutcome = GrugSignatureOutcome<JsonValue, Credential>;

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
