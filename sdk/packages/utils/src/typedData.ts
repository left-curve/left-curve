import type {
  Coins,
  EIP712Domain,
  EIP712Message,
  Hex,
  Json,
  Message,
  Metadata,
  Power,
  TxMessageType,
  TypedData,
  TypedDataParameter,
  TypedDataProperty,
  Username,
} from "@left-curve/types";
import type { HashTypedDataParameters } from "viem";
import { recursiveTransform } from "./mappers.js";
import { camelToSnake } from "./strings.js";

/**
 * @description Hash the typed data.
 *
 * @param typedData The typed data to hash.
 * @returns The hashed typed data.
 */
export async function hashTypedData(
  typedData: HashTypedDataParameters<Record<string, unknown>, string>,
): Promise<Hex> {
  const { hashTypedData: viemHashTypedData } = await import("viem");

  return viemHashTypedData(typedData);
}

export type ArbitraryTypedDataParameters = {
  message: Json;
  types: Record<string, TypedDataProperty[]>;
  primaryType: string;
};

/**
 * @description Composes arbitrary typed data.
 *
 * @params parameters The parameters to compose the typed data.
 * @params parameters.message The typed message.
 * @params parameters.types The typed data types.
 * @params parameters.primaryType The primary type.
 * @returns The composed typed data.
 */
export function composeArbitraryTypedData(parameters: ArbitraryTypedDataParameters) {
  const { message, types, primaryType } = parameters;
  return {
    domain: {
      name: "DangoArbitraryMessage",
    },
    message: recursiveTransform(message, camelToSnake) as Record<string, unknown>,
    primaryType,
    types: {
      EIP712Domain: [{ name: "name", type: "string" }],
      ...types,
    },
  };
}

/**
 * @description Composes the typed data for a transaction.
 *
 * @param message The typed message.
 * @param typeData The typed data parameters.
 * @retuns The composed typed data
 */
export function composeTxTypedData(
  message: EIP712Message,
  domain: EIP712Domain,
  typeData?: Partial<TypedDataParameter<TxMessageType>>,
): TypedData {
  const { type = [], extraTypes = {} } = typeData || {};
  const { messages, metadata, gas_limit } = message;
  const { expiry } = metadata;

  return {
    types: {
      EIP712Domain: [
        { name: "name", type: "string" },
        { name: "verifyingContract", type: "address" },
      ],
      Message: [
        { name: "metadata", type: "Metadata" },
        { name: "gas_limit", type: "uint32" },
        { name: "messages", type: "TxMessage[]" },
      ],
      Metadata: [
        { name: "username", type: "string" },
        { name: "chain_id", type: "string" },
        { name: "nonce", type: "uint32" },
        ...(metadata.expiry ? [{ name: "expiry", type: "string" }] : []),
      ],
      TxMessage: type,
      ...extraTypes,
    },
    primaryType: "Message",
    domain,
    message: {
      metadata: recursiveTransform(
        { ...metadata, ...(expiry ? { expiry } : {}) },
        camelToSnake,
      ) as Metadata,
      gas_limit,
      messages: recursiveTransform(messages, camelToSnake) as Message[],
    },
  };
}

/**
 * @description Gets the typed data for coins.
 *
 * @param coins The coins to get the typed data for.
 * @returns The typed data properties.
 */
export function getCoinsTypedData(coins?: Coins): TypedDataProperty[] {
  if (!coins) return [];
  return Object.keys(coins).map((coin) => ({ name: coin, type: "string" }));
}

/**
 * @description Gets the typed data for members.
 *
 * @param members The members to get the typed data for.
 * @returns The typed data properties.
 */
export function getMembersTypedData(members?: Record<Username, Power>): TypedDataProperty[] {
  if (!members) return [];
  return Object.keys(members).map((member) => ({ name: member, type: "uint32" }));
}
