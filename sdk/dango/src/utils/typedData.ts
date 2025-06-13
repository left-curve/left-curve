import { camelToSnake, recursiveTransform } from "@left-curve/sdk/utils";

import type { Coins, Json, Message } from "@left-curve/sdk/types";

import type {
  ArbitraryTypedData,
  EIP712Domain,
  EIP712Message,
  TxMessageType,
  TypedData,
  TypedDataParameter,
  TypedDataProperty,
} from "../types/typedData.js";

/**
 * @description Composes arbitrary typed data.
 *
 * @param parameters The parameters to compose the typed data.
 * @param parameters.message The typed message.
 * @param parameters.types The typed data types.
 * @param parameters.primaryType The primary type.
 * @returns The composed typed data.
 */
export function composeArbitraryTypedData(parameters: ArbitraryTypedData) {
  const { message, types, primaryType } = parameters;
  return {
    domain: {
      name: "DangoArbitraryMessage",
      chainId: 1,
      verifyingContract: "0x0000000000000000000000000000000000000000",
    },
    message: recursiveTransform(message, camelToSnake) as Record<string, unknown>,
    primaryType,
    types: {
      EIP712Domain: [
        { name: "name", type: "string" },
        { name: "chainId", type: "uint256" },
        { name: "verifyingContract", type: "address" },
      ],
      ...types,
    },
  };
}

/**
 * @description Composes the typed data for a transaction.
 *
 * @param message The typed message.
 * @param typeData The typed data parameters.
 * @returns The composed typed data
 */
export function composeTxTypedData(
  message: EIP712Message,
  domain: EIP712Domain,
  typeData?: Partial<TypedDataParameter<TxMessageType>>,
): TypedData<TxMessageType> {
  const { type = [], extraTypes = {} } = typeData || {};
  const { messages, data, gas_limit, sender } = message;
  const { expiry } = data;

  return {
    types: {
      EIP712Domain: [
        { name: "name", type: "string" },
        { name: "chainId", type: "uint256" },
        { name: "verifyingContract", type: "address" },
      ],
      Message: [
        { name: "sender", type: "address" },
        { name: "data", type: "Metadata" },
        { name: "gas_limit", type: "uint32" },
        { name: "messages", type: "TxMessage[]" },
      ],
      Metadata: [
        { name: "username", type: "string" },
        { name: "chain_id", type: "string" },
        { name: "nonce", type: "uint32" },
        ...(data.expiry ? [{ name: "expiry", type: "string" }] : []),
      ],
      TxMessage: type,
      ...extraTypes,
    },
    primaryType: "Message",
    domain,
    message: {
      sender,
      data: recursiveTransform({ ...data, ...(expiry ? { expiry } : {}) }, camelToSnake) as Json,
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
export function getMembersTypedData(members?: Record<string, number>): TypedDataProperty[] {
  if (!members) return [];
  return Object.keys(members).map((member) => ({ name: member, type: "uint32" }));
}
