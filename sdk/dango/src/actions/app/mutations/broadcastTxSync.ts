import { encodeBase64, serialize } from "@left-curve/sdk/encoding";
import type { Prettify, Transport, Tx, TxData, UnsignedTx } from "@left-curve/sdk/types";

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

  const txBase64 = encodeBase64(serialize(tx));

  const result = await (async () => {
    if (transport.type !== "http-graphql") {
      return await transport.request({
        method: "broadcast_tx_sync",
        params: {
          tx: txBase64,
        },
      });
    }

    const document = `
      mutation broadcastTxSyncResult($tx: String!) {
          broadcastTxSync(tx: $tx) {
            hash
            log
            code
          }
        }
    `;

    const response = await queryIndexer(client, {
      document,
      variables: { tx: txBase64 },
    });

    const { broadcastTxSync: result } = response as unknown as {
      broadcastTxSync: TxData & { hash: Uint8Array };
    };

    return result;
  })();

  const { code, codespace, log } = result;

  if (code === 1) {
    throw new Error(`failed to broadcast tx! codespace: ${codespace}, code: ${code}, log: ${log}`);
  }

  await withRetry(
    ({ abort }) =>
      async () => {
        {
          const tx = await queryTx(client, {
            hash: typeof result.hash === "string" ? result.hash : encodeBase64(result.hash),
          });
          if (!tx) throw new Error("Transaction not found");

          if (tx.tx_result.code === 1) {
            const { codespace, code, log } = tx.tx_result;
            abort(tx.tx_result.data || "Transaction failed");
          }
          return tx;
        }
      },
    { delay: 500, retryCount: 30 },
  );

  return result;
}
