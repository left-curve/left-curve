import { camelToSnake, recursiveTransform } from "./index.js";

import type {
  Address,
  Json,
  JsonValue,
  Message,
  OnboardDoc,
  OnboardTypedData,
  SessionDoc,
  SessionTypedData,
  SignDoc,
  TxTypedData,
} from "@left-curve/types";

const DOMAIN_NAME = "dango";
const PRIMARY_TYPE = "Message";
const ZERO_ADDRESS: Address = "0x0000000000000000000000000000000000000000";
const EIP712_DOMAIN_TYPE = [
  { name: "name", type: "string" },
  { name: "chainId", type: "uint256" },
  { name: "verifyingContract", type: "address" },
] as const;

const MESSAGE_KINDS = [
  "configure",
  "upgrade",
  "transfer",
  "upload",
  "instantiate",
  "execute",
  "migrate",
] as const;

type MessageKind = (typeof MESSAGE_KINDS)[number];

/**
 * @description Composes the canonical EIP-712 typed data for a transaction.
 *
 * The schema collapses each `Message` enum variant to `{kind, payload}` where
 * `payload` is the canonical JSON string of the inner value. The Dango
 * verifier rebuilds this shape from the parsed `SignDoc`, so the signed
 * digest matches Rust byte-for-byte.
 */
export function composeTxTypedData(signDoc: SignDoc): TxTypedData {
  const { sender, gasLimit, messages, data } = signDoc;
  const { chainId, userIndex, nonce, expiry } = data;

  const txMessages = messages.map((message) => {
    const [kind, value] = entryOf(message);
    if (!isMessageKind(kind)) {
      throw new Error(`unknown message kind: ${kind}`);
    }
    return {
      kind,
      payload: canonicalJson(recursiveTransform(value as JsonValue, camelToSnake)),
    };
  });

  return {
    types: {
      EIP712Domain: [...EIP712_DOMAIN_TYPE],
      Message: [
        { name: "sender", type: "address" },
        { name: "data", type: "Metadata" },
        // `gas_limit` is `string` rather than `uint64` so the JSON value is
        // also a string, avoiding JS `Number` precision loss above 2^53.
        { name: "gas_limit", type: "string" },
        { name: "messages", type: "TxMessage[]" },
      ],
      Metadata: [
        { name: "user_index", type: "uint32" },
        { name: "chain_id", type: "string" },
        { name: "nonce", type: "uint32" },
        ...(expiry !== undefined ? [{ name: "expiry", type: "string" }] : []),
      ],
      TxMessage: [
        { name: "kind", type: "string" },
        { name: "payload", type: "string" },
      ],
    },
    primaryType: PRIMARY_TYPE,
    domain: {
      name: DOMAIN_NAME,
      chainId: 1,
      verifyingContract: sender,
    },
    message: {
      sender,
      data: {
        user_index: userIndex,
        chain_id: chainId,
        nonce,
        ...(expiry !== undefined ? { expiry } : {}),
      },
      gas_limit: String(gasLimit),
      messages: txMessages,
    },
  };
}

/**
 * @description Composes the canonical EIP-712 typed data for a session.
 */
export function composeSessionTypedData(doc: SessionDoc): SessionTypedData {
  const { chainId, sessionKey, expireAt } = doc;

  return {
    types: {
      EIP712Domain: [...EIP712_DOMAIN_TYPE],
      Message: [
        { name: "chain_id", type: "string" },
        { name: "session_key", type: "string" },
        { name: "expire_at", type: "string" },
      ],
    },
    primaryType: PRIMARY_TYPE,
    domain: {
      name: DOMAIN_NAME,
      chainId: 1,
      verifyingContract: ZERO_ADDRESS,
    },
    message: {
      chain_id: chainId,
      session_key: sessionKey,
      expire_at: expireAt,
    },
  };
}

/**
 * @description Composes the canonical EIP-712 typed data for user onboarding.
 */
export function composeOnboardTypedData(doc: OnboardDoc): OnboardTypedData {
  const { chainId, key, keyHash, seed, referrer } = doc;

  return {
    types: {
      EIP712Domain: [...EIP712_DOMAIN_TYPE],
      Message: [
        { name: "chain_id", type: "string" },
        { name: "key", type: "string" },
        { name: "key_hash", type: "string" },
        { name: "seed", type: "uint32" },
        ...(referrer !== undefined ? [{ name: "referrer", type: "uint32" }] : []),
      ],
    },
    primaryType: PRIMARY_TYPE,
    domain: {
      name: DOMAIN_NAME,
      chainId: 1,
      verifyingContract: ZERO_ADDRESS,
    },
    message: {
      chain_id: chainId,
      key: canonicalJson(key as JsonValue),
      key_hash: keyHash,
      seed,
      ...(referrer !== undefined ? { referrer } : {}),
    },
  };
}

/**
 * @description Serialize a JSON value to a canonical string, matching the
 * Rust `serde_json` byte output: object keys sorted alphabetically, no
 * whitespace, standard JSON escaping.
 */
export function canonicalJson(value: JsonValue): string {
  return JSON.stringify(canonicalize(value));
}

function canonicalize(value: JsonValue): JsonValue {
  if (value === null || typeof value !== "object") return value;
  if (Array.isArray(value)) return value.map(canonicalize);
  const sorted: Json = {};
  for (const key of Object.keys(value).sort()) {
    sorted[key] = canonicalize((value as Json)[key] as JsonValue);
  }
  return sorted;
}

function entryOf(message: Message): [string, unknown] {
  const entries = Object.entries(message);
  if (entries.length !== 1) {
    throw new Error("message must have exactly one variant");
  }
  return entries[0] as [string, unknown];
}

function isMessageKind(value: string): value is MessageKind {
  return (MESSAGE_KINDS as readonly string[]).includes(value);
}
