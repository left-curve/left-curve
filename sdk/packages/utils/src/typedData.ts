import type {
  Coins,
  Hex,
  MessageTypedDataType,
  Power,
  TxMessageTypedDataType,
  TypedData,
  TypedDataParameter,
  TypedDataProperties,
  Username,
} from "@leftcurve/types";

/**
 * @description Hash the typed data.
 *
 * @param typedData The typed data to hash.
 * @returns The hashed typed data.
 */
export async function hashTypedData(typedData: TypedData): Promise<Hex> {
  const { TypedDataEncoder } = await import("ethers");
  return TypedDataEncoder.hash(typedData.domain, typedData.types, typedData.message);
}

/**
 * @description Composes the typed data for a transaction.
 *
 * @param message The typed message.
 * @param typeData The typed data parameters.
 * @retuns The composed typed data
 */
export function composeTypedData(
  message: TxMessageTypedDataType,
  typeData?: Partial<TypedDataParameter<MessageTypedDataType>>,
): TypedData {
  const { type = [], extraTypes = {} } = typeData || {};

  return {
    types: {
      Tx: [
        { name: "chainId", type: "string" },
        { name: "sequence", type: "uint32" },
        { name: "messages", type: "TxMessage[]" },
      ],
      TxMessage: type,
      ...extraTypes,
    },
    primaryType: "Tx",
    domain: {},
    message,
  };
}

/**
 * @description Gets the typed data for coins.
 *
 * @param coins The coins to get the typed data for.
 * @returns The typed data properties.
 */
export function getCoinsTypedData(coins?: Coins): TypedDataProperties[] {
  if (!coins) return [];
  return Object.keys(coins).map((coin) => ({ name: coin, type: "string" }));
}

/**
 * @description Gets the typed data for members.
 *
 * @param members The members to get the typed data for.
 * @returns The typed data properties.
 */
export function getMembersTypedData(members?: Record<Username, Power>): TypedDataProperties[] {
  if (!members) return [];
  return Object.keys(members).map((member) => ({ name: member, type: "uint32" }));
}
