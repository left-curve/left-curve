import { queryTx as internalQueryTx } from "@left-curve/sdk/actions";

import type { QueryTxParameters, QueryTxReturnType } from "@left-curve/sdk/actions";
import type { Client, Transport } from "@left-curve/sdk/types";
import type { Chain } from "../../../types/chain.js";
import type { Signer } from "../../../types/signer.js";
import { queryIndexer } from "../../indexer/queryIndexer.js";

/**
 * Query the application state.
 * @param parameters
 * @param parameters.query The query request.
 * @param parameters.height The height at which to query the application state.
 * @returns The query response.
 */
export async function queryTx<
  chain extends Chain | undefined = Chain,
  signer extends Signer | undefined = undefined,
>(client: Client<Transport, chain, signer>, parameters: QueryTxParameters): QueryTxReturnType {
  const { hash } = parameters;
  const { transport } = client;

  if (transport.type !== "http-graphql") return await internalQueryTx(client, parameters);

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
      }[];
    };
  };

  const { transactions } = await queryIndexer<TxReturnType, chain, signer>(client, {
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
    },
  };
}
