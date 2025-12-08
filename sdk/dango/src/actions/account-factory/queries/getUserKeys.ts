import { queryIndexer } from "../../indexer/queryIndexer.js";

import type { Client, Transport } from "@left-curve/sdk/types";
import type { PublicKey } from "../../../types/key.js";

export type GetUserKeysParameters = {
  userIndex: number;
};

export type GetUserKeysReturnType = Promise<PublicKey[]>;

export async function getUserKeys<transport extends Transport>(
  client: Client<transport>,
  parameters: GetUserKeysParameters,
): GetUserKeysReturnType {
  const document = /* GraphQL */ `
   query keys($userIndex: Int!){
    user(userIndex: $userIndex) {
        publicKeys {
        id
        keyHash
        publicKey
        keyType
        createdBlockHeight
        createdAt
        }
      }
    }`;

  const { user } = await queryIndexer<{ user: { publicKeys: PublicKey[] } }>(client, {
    document,
    variables: parameters,
  });

  return user.publicKeys;
}
