import type { Address, Json } from "./index.js";

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
  { name: "gas_limit"; type: "string" },
  { name: "messages"; type: "TxMessage[]" },
];

export type MetadataType = readonly TypedDataProperty[];

export type TxMessageType = [{ name: "kind"; type: "string" }, { name: "payload"; type: "string" }];

export type EIP712Domain = {
  name: string;
  chainId: number;
  verifyingContract: Address;
};

/** EIP-712 typed data shared across all flavors (tx, session, onboard). */
export type TypedData<
  TTypes extends Record<string, readonly TypedDataProperty[]> = Record<
    string,
    readonly TypedDataProperty[]
  >,
  TMessage extends Json = Json,
> = {
  types: TTypes & { EIP712Domain: DomainType };
  primaryType: "Message";
  domain: EIP712Domain;
  message: TMessage;
};

/** Typed data for a Dango transaction (`SignDoc`). */
export type TxTypedData = TypedData<
  {
    EIP712Domain: DomainType;
    Message: MessageType;
    Metadata: MetadataType;
    TxMessage: TxMessageType;
  },
  TxTypedDataMessage
>;

export type TxTypedDataMessage = {
  sender: Address;
  data: TxTypedDataMetadata;
  gas_limit: string;
  messages: TxTypedDataEntry[];
};

export type TxTypedDataMetadata = {
  user_index: number;
  chain_id: string;
  nonce: number;
  expiry?: string;
};

export type TxTypedDataEntry = {
  kind: string;
  payload: string;
};

/** Typed data for a session authorization (`SessionInfo`). */
export type SessionTypedData = TypedData<
  {
    EIP712Domain: DomainType;
    Message: readonly TypedDataProperty[];
  },
  SessionTypedDataMessage
>;

export type SessionTypedDataMessage = {
  chain_id: string;
  session_key: string;
  expire_at: string;
};

/** Typed data for user onboarding (`RegisterUserData`). */
export type OnboardTypedData = TypedData<
  {
    EIP712Domain: DomainType;
    Message: readonly TypedDataProperty[];
  },
  OnboardTypedDataMessage
>;

export type OnboardTypedDataMessage = {
  chain_id: string;
  key: string;
  key_hash: string;
  seed: number;
  referrer?: number;
};

/** Typed data passed through the arbitrary signing API. */
export type ArbitraryTypedData = SessionTypedData | OnboardTypedData;
