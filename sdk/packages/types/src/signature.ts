import type { Address } from "./address.js";
import type { Hex, Json, JsonValue } from "./encoding.js";
import type { Message } from "./tx.js";

export type SignDoc<Metadata = Json> = {
  sender: Address;
  messages: Message[];
  gasLimit: number;
  data: Metadata;
};

export type SignatureOutcome<Metadata = Json, Credential = Json> = {
  credential: Credential;
  signDoc: SignDoc<Metadata>;
};

export type ArbitrarySignatureOutcome<Credential = Json> = {
  credential: Credential;
  payload: JsonValue;
};

export type RawSignature = {
  r: Hex;
  s: Hex;
  v: number;
};
