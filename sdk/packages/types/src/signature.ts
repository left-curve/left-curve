import type { Address } from "./address";
import type { Credential } from "./credential";
import type { Hex } from "./encoding";
import type { KeyHash } from "./key";
import type { Message } from "./tx";
import type { TxMessageType, TypedDataParameter } from "./typedData";

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
