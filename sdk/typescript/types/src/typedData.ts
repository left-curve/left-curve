import type { Address, Json, JsonValue, Message } from "./index.js";

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
  { name: "chainId"; type: "uint256" },
  { name: "verifyingContract"; type: "address" },
];

export type MessageType = [
  { name: "sender"; type: "address" },
  { name: "data"; type: "Metadata" },
  { name: "gas_limit"; type: "uint32" },
  { name: "messages"; type: "string[]" },
];

export type MetadataType = [
  { name: "user_index"; type: "uint32" },
  { name: "chain_id"; type: "string" },
  { name: "nonce"; type: "uint32" },
];

/**
 * EIP-712 has no sum type, so a transaction's `messages` can't be expressed as
 * structs; they are bound as canonical JSON strings instead (`Message.messages`
 * is typed `string[]`, and the `eip1193` signer stringifies the values). This
 * per-variant type is therefore no longer used to build the transaction typed
 * data -- it remains only for the vestigial `typedData` parameters still
 * accepted (and ignored) by the action mutations, pending their removal.
 */
export type TxMessageType =
  | { name: "configure"; type: "Configure" }
  | { name: "upgrade"; type: "Upgrade" }
  | { name: "transfer"; type: "Transfer" }
  | { name: "upload"; type: "Upload" }
  | { name: "instantiate"; type: "Instantiate" }
  | { name: "execute"; type: "Execute" }
  | { name: "migrate"; type: "Migrate" };

export type TypedData = {
  types: EIP712Types;
  primaryType: "Message";
  domain: EIP712Domain;
  message: EIP712Message;
};

export type EIP712Types = Record<"Message", MessageType> &
  Record<"Metadata", TypedDataProperty[]> &
  Record<"EIP712Domain", DomainType>;

export type EIP712Message = {
  sender: Address;
  data: Json;
  gas_limit: number;
  messages: Message[];
};

export type EIP712Domain = {
  name: string;
  chainId: number;
  verifyingContract: Address;
};

export type ArbitraryTypedData<message extends JsonValue = JsonValue> = {
  message: message;
  types: Record<string, TypedDataProperty[]>;
  primaryType: "Message";
};
