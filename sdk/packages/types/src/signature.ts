import type { Credential } from "./credential";
import type { Hex } from "./encoding";
import type { KeyHash } from "./key";
import type { Message } from "./tx";
import type { MessageTypedDataType, TypedDataParameter } from "./typedData";

export type EthPersonalMessage = Hex | string | Uint8Array;

export type Signature = {
  r: Hex;
  s: Hex;
  v: number;
};

export type SignDoc = {
  messages: Message[];
  chainId: string;
  sequence: number;
  typedData?: TypedDataParameter<MessageTypedDataType>;
};

export type SignedDoc = {
  credential: Credential;
  keyHash: KeyHash;
  signDoc: SignDoc;
};
