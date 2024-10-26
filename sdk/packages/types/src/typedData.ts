import type { Address } from "./address";
import type { Message } from "./tx";

// biome-ignore format: no formatting
type MBytes =
  | '' | 1  | 2  | 3  | 4  | 5  | 6  | 7  | 8  | 9
  | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19
  | 20 | 21 | 22 | 23 | 24 | 25 | 26 | 27 | 28 | 29
  | 30 | 31 | 32
// biome-ignore format: no formatting
type MBits =
  | ''  | 8   | 16  | 24  | 32  | 40  | 48  | 56  | 64  | 72
  | 80  | 88  | 96  | 104 | 112 | 120 | 128 | 136 | 144 | 152
  | 160 | 168 | 176 | 184 | 192 | 200 | 208 | 216 | 224 | 232
  | 240 | 248 | 256

export type TypedDataProperty = {
  name: string;
  type: SolidityTypes;
};

export type TypedDataParameter<T = TypedDataProperty> = {
  type: T[];
  extraTypes: Record<string, TypedDataProperty[]>;
};

export type SolidityTypes =
  | "string"
  | "address"
  | "bool"
  | `${"u" | ""}int${MBits}`
  | `bytes${MBytes}`
  | string
  | `${string}[]`
  | `${string}[${number}]`;

export type DomainType = [
  { name: "name"; type: "string" },
  { name: "verifyingContract"; type: "address" },
];

export type MessageType = [
  { name: "chainId"; type: "string" },
  { name: "sequence"; type: "uint32" },
  { name: "messages"; type: "TxMessage[]" },
];

export type TxMessageType =
  | { name: "configure"; type: "Configure" }
  | { name: "transfer"; type: "Transfer" }
  | { name: "upload"; type: "Upload" }
  | { name: "instantiate"; type: "Instantiate" }
  | { name: "execute"; type: "Execute" }
  | { name: "migrate"; type: "Migrate" };

export type TypedData<TType extends TxMessageType | unknown = TxMessageType | unknown> = {
  types: EIP712Types<TType>;
  primaryType: "Message";
  domain: EIP712Domain;
  message: EIP712Message;
};

export type EIP712Types<TMessage extends TxMessageType | unknown = TxMessageType | unknown> =
  Record<"Message", MessageType> &
    Record<"TxMessage", TMessage[]> &
    Record<"EIP712Domain", DomainType>;

export type EIP712Message = {
  chainId: string;
  sequence: number;
  messages: Message[];
};

export type EIP712Domain = {
  name: string;
  verifyingContract: Address;
};
