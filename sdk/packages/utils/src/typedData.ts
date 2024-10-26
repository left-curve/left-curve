import type {
  Coins,
  EIP712Domain,
  EIP712Message,
  EIP712Types,
  Hex,
  Power,
  TxMessageType,
  TypedData,
  TypedDataParameter,
  TypedDataProperty,
  Username,
} from "@leftcurve/types";

/**
 * @description Hash the typed data.
 *
 * @param typedData The typed data to hash.
 * @returns The hashed typed data.
 */
export async function hashTypedData(typedData: TypedData): Promise<Hex> {
  const { hashTypedData: viemHashTypedData } = await import("viem");

  return viemHashTypedData<EIP712Types, "Message">(typedData);
}

/**
 * @description Composes the typed data for a transaction.
 *
 * @param message The typed message.
 * @param typeData The typed data parameters.
 * @retuns The composed typed data
 */
export function composeTypedData(
  message: EIP712Message,
  domain: EIP712Domain,
  typeData?: Partial<TypedDataParameter<TxMessageType>>,
): TypedData {
  const { type = [], extraTypes = {} } = typeData || {};

  return {
    types: {
      EIP712Domain: [
        { name: "name", type: "string" },
        { name: "verifyingContract", type: "address" },
      ],
      Message: [
        { name: "chainId", type: "string" },
        { name: "sequence", type: "uint32" },
        { name: "messages", type: "TxMessage[]" },
      ],
      TxMessage: type,
      ...extraTypes,
    },
    primaryType: "Message",
    domain,
    message,
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
