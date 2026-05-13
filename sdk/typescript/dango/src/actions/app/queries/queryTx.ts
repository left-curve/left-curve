import type { Client } from "../../../types/index.js";
import type { Base64, TxResponse } from "../../../types/index.js";
import { queryIndexer } from "../../indexer/queryIndexer.js";

export type QueryTxParameters = {
  hash: Base64;
};

export type QueryTxReturnType = Promise<TxResponse | null>;

export async function queryTx(client: Client, parameters: QueryTxParameters): QueryTxReturnType {
  const { hash } = parameters;

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
      nodes: {
        hash: string;
        blockHeight: string;
        transactionIdx: number;
        hasSucceeded: string;
        sender: string;
        nestedEvents: string;
        transactionType: string;
        gasUsed: string;
        gasWanted: string;
        errorMessage: string;
      }[];
    };
  };

  const { transactions } = await queryIndexer<TxReturnType>(client, {
    document,
    variables: { hash },
  });

  const [indexedTx] = transactions.nodes;

  if (!indexedTx) return null;

  return {
    hash: indexedTx.hash,
    height: indexedTx.blockHeight,
    index: indexedTx.transactionIdx,
    tx: indexedTx.transactionType,
    tx_result: {
      code: indexedTx.hasSucceeded ? 0 : 1,
      gas_used: indexedTx.gasUsed,
      gas_wanted: indexedTx.gasWanted,
      log: indexedTx.nestedEvents,
      data: indexedTx.errorMessage,
    },
  };
}
