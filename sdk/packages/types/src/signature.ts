import type { Address } from "./address.js";
import type { Credential } from "./credential.js";
import type { Hex } from "./encoding.js";
import type { KeyHash } from "./key.js";
import type { Message } from "./tx.js";
import type { TxMessageType, TypedDataParameter } from "./typedData.js";

export type EthPersonalMessage = Hex | string | Uint8Array;

export type Signature = {
  r: Hex;
  s: Hex;
  v: number;
};

export type SignDoc = {
  sender: Address;
  messages: Message[];
  chainId: string;
  sequence: number;
  typedData?: TypedDataParameter<TxMessageType>;
};

export type SignedDoc = {
  credential: Credential;
  keyHash: KeyHash;
  signDoc: SignDoc;
};
