import { queryIndexer } from "#actions/indexer/queryIndexer.js";

import type { Client, PublicKey } from "@left-curve/types";

export type GetUserKeysParameters = {
  userIndex: number;
};

export type GetUserKeysReturnType = Promise<PublicKey[]>;

export async function getUserKeys(
  client: Client,
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
