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

export type TypedDataProperties = {
  name: string;
  type: TypedDataTypes;
};

export type TypedDataParameter<T = TypedDataProperties> = {
  type: T[];
  extraTypes: Record<string, TypedDataProperties[]>;
};

export type TxMessageTypedDataType = {
  chainId: string;
  sequence: number;
  messages: Message[];
};

export type TypedDataTypes =
  | "string"
  | "address"
  | "bool"
  | `${"u" | ""}int${MBits}`
  | `bytes${MBytes}`
  | string
  | `${string}[]`
  | `${string}[${number}]`;

export type TxTypedDataType = [
  { name: "chainId"; type: "string" },
  { name: "sequence"; type: "uint32" },
  { name: "messages"; type: "TxMessage[]" },
];

export type MessageTypedDataType =
  | { name: "configure"; type: "Configure" }
  | { name: "transfer"; type: "Transfer" }
  | { name: "upload"; type: "Upload" }
  | { name: "instantiate"; type: "Instantiate" }
  | { name: "execute"; type: "Execute" }
  | { name: "migrate"; type: "Migrate" };

export type TypedData = {
  types: Record<"Message", TxTypedDataType> & Record<"TxMessage", MessageTypedDataType[]>;
  primaryType: "Message";
  domain: Record<string, never>;
  message: TxMessageTypedDataType;
};
