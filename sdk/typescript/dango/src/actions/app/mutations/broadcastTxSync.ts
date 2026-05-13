import { camelCaseJsonDeserialization, snakeCaseJsonSerialization } from "@left-curve/encoding";
import type { Client, Prettify, Tx, TxData, UnsignedTx } from "@left-curve/types";
import { withRetry } from "@left-curve/utils";
import { queryIndexer } from "#actions/indexer/queryIndexer.js";
import { queryTx } from "../queries/queryTx.js";

export type BroadcastTxSyncParameters = {
  tx: Tx | UnsignedTx;
};

export type BroadcastTxSyncReturnType = Promise<Prettify<{ hash: Uint8Array } & TxData>>;

export async function broadcastTxSync(
  client: Client,
  parameters: BroadcastTxSyncParameters,
): BroadcastTxSyncReturnType {
  const { tx } = parameters;

  const document = `
    mutation broadcastTxSyncResult($tx: String!) {
        broadcastTxSync(tx: $tx)
    }
  `;

  const { broadcastTxSync } = await queryIndexer<{
    broadcastTxSync: {
      txHash: string;
      checkTx: {
        gaslimit: number;
        gasUsed: number;
        result: { Ok: null } | { Error: string };
      };
    };
  }>(client, {
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

  const resultKey = Object.keys(checkTx.result)[0];

  const result = {
    code: resultKey === "Ok" ? 0 : 1,
    log: checkTx.result[resultKey as keyof typeof checkTx.result],
    hash: txHash,
  };

  const { code, log } = result;

  if (code === 1) {
    const logStr = typeof log === "string" ? log : JSON.stringify(log);
    throw new Error(`failed to broadcast tx! code: ${code}, log: ${logStr}`);
  }

  await withRetry(
    ({ abort }) =>
      async () => {
        {
          const hash = typeof result.hash === "string" ? result.hash : result.hash;
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
