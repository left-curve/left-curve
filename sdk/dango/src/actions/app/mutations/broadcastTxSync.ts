import {
  camelCaseJsonDeserialization,
  encodeBase64,
  serialize,
  snakeCaseJsonSerialization,
} from "@left-curve/sdk/encoding";
import type { Chain, Prettify, Transport, Tx, TxData, UnsignedTx } from "@left-curve/sdk/types";

import { withRetry } from "@left-curve/sdk/utils";
import type { DangoClient, Signer } from "../../../types/index.js";
import { queryIndexer } from "../../indexer/queryIndexer.js";
import { queryTx } from "../queries/queryTx.js";

export type BroadcastTxSyncParameters = {
  tx: Tx | UnsignedTx;
};

export type BroadcastTxSyncReturnType = Promise<Prettify<{ hash: Uint8Array } & TxData>>;

/**
 * Broadcasts a transaction synchronously.
 * @param parameters
 * @param parameters.tx The transaction to broadcast.
 * @returns The transaction hash and data.
 */
export async function broadcastTxSync<transport extends Transport>(
  client: DangoClient<transport, undefined | Signer>,
  parameters: BroadcastTxSyncParameters,
): BroadcastTxSyncReturnType {
  const { tx } = parameters;
  const { transport } = client;

  const result = await (async () => {
    if (transport.type !== "http-graphql") {
      return await transport.request({
        method: "broadcast_tx_sync",
        params: {
          tx: encodeBase64(serialize(tx)),
        },
      });
    }

    const document = `
      mutation broadcastTxSyncResult($tx: String!) {
          broadcastTxSync(tx: $tx)
      }
    `;

    const { broadcastTxSync } = await queryIndexer<
      {
        broadcastTxSync: {
          txHash: string;
          checkTx: {
            gaslimit: number;
            gasUsed: number;
            result: { Ok: null } | { Error: string };
          };
        };
      },
      Chain,
      Signer | undefined
    >(client, {
      document,
      variables: { tx: snakeCaseJsonSerialization(tx) },
    });
    const { checkTx, txHash } = camelCaseJsonDeserialization(broadcastTxSync) as {
      txHash: string;
      checkTx: {
        gaslimit: number;
        gasUsed: number;
        result: { Ok: null } | { Error: string };
      };
    };

    const result = Object.keys(checkTx.result)[0];

    return {
      code: result === "Ok" ? 0 : 1,
      log: checkTx.result[result as keyof typeof checkTx.result],
      hash: txHash,
    };
  })();

  const { code, log } = result;

  if (code === 1) {
    throw new Error(`failed to broadcast tx! code: ${code}, log: ${log}`);
  }

  await withRetry(
    ({ abort }) =>
      async () => {
        {
          const hash = typeof result.hash === "string" ? result.hash : encodeBase64(result.hash);
          const tx = await queryTx(client, {
            hash,
          });
          if (!tx) throw new Error(`Transaction not found: ${hash}`);

          if (tx.tx_result.code === 1) {
            abort(tx.tx_result.data || "Transaction failed");
          }
          return tx;
        }
      },
    { delay: 500, retryCount: 30 },
  );

  return result as unknown as BroadcastTxSyncReturnType;
}
