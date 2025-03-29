import type { Client, Transport } from "@left-curve/sdk/types";
import type { IndexedTransaction } from "../../types/indexer.js";
import { queryIndexer } from "./queryIndexer.js";

export type SearchTxParameters = {
  hash: string;
};

export type SearchTxReturnType = Promise<IndexedTransaction | null>;

export async function searchTx<transport extends Transport>(
  client: Client<transport>,
  parameters: SearchTxParameters,
): SearchTxReturnType {
  const document = `
    query tx($hash: String!) {
      transactions(hash: $hash) {
        nodes {
          hash
          blockHeight
          transactionIdx
          hasSucceeded
          sender
          nestedEvents
          transactionType
          createdAt
          gasWanted
          gasUsed
          errorMessage
        }
      }
    }
  `;

  type TxReturnType = {
    transactions: {
      nodes: IndexedTransaction[];
    };
  };

  const { transactions } = await queryIndexer<TxReturnType>(client, {
    document,
    variables: parameters,
  });

  const [tx] = transactions.nodes;

  if (!tx) return null;

  return tx;
}
