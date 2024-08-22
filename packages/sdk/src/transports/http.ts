import { Comet38Client, HttpBatchClient, HttpClient } from "@cosmjs/tendermint-rpc";
import type { AbciQueryResponse } from "@cosmjs/tendermint-rpc/build/comet38";
import { serialize } from "@leftcurve/encoding";
import type { Hex, Transport, Tx, UnsignedTx } from "@leftcurve/types";
import { UrlRequiredError } from "../errors/transports";
import { createTransport } from "./createTransport";

export type HttpTransportConfig = {
  /**
   * Whether to enable Batch JSON-RPC.
   * @link https://www.jsonrpc.org/specification#batch
   */
  batch?:
    | boolean
    | {
        /** Interval for dispatching batches (in milliseconds) */
        dispatchInterval: number;
        /** Max number of items sent in one request */
        sizeLimit: number;
      };
  /** The name of the transport. */
  name?: string;
  /** The key of the transport. */
  key?: string;
};

export type HttpTransport = Transport<"http">;

/**
 * Creates a HTTP transport that connects to a JSON-RPC API.
 * @param url The URL of the JSON-RPC API.
 * @param config The configuration of the transport.
 * @returns The HTTP transport.
 */
export function http(url?: string | undefined, config: HttpTransportConfig = {}): HttpTransport {
  const { batch, key = "http", name = "HTTP JSON-RPC" } = config;
  return ({ chain }) => {
    const url_ = url || chain?.rpcUrls.default.http[0];
    if (!url_) throw new UrlRequiredError();

    const rpcClient = batch
      ? new HttpBatchClient(
          url_,
          typeof batch === "object" ? batch : { sizeLimit: 20, dispatchInterval: 20 },
        )
      : new HttpClient(url_);

    // @ts-ignore
    const cometClient = new Comet38Client(rpcClient);

    async function query(
      path: string,
      data: Uint8Array,
      height = 0,
      prove = false,
    ): Promise<AbciQueryResponse> {
      const res = await cometClient.abciQuery({
        path,
        data,
        height,
        prove,
      });

      if (res.code === 0) return res;
      throw new Error(
        `query failed! codespace: ${res.codespace}, code: ${res.code}, log: ${res.log}`,
      );
    }

    async function broadcast(tx: Tx | UnsignedTx): Promise<Hex> {
      const { code, codespace, log, hash } = await cometClient.broadcastTxSync({
        tx: serialize(tx),
      });
      if (code === 0) return hash;
      throw new Error(
        `failed to broadcast tx! codespace: ${codespace}, code: ${code}, log: ${log}`,
      );
    }

    return createTransport(
      {
        key,
        name,
        type: "http",
      },
      {
        query,
        broadcast,
      },
    );
  };
}
