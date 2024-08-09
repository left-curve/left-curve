import { Comet38Client, type HttpEndpoint, type RpcClient } from "@cosmjs/tendermint-rpc";
import { decodeBase64, decodeHex, deserialize, encodeBase64, serialize } from "@leftcurve/encoding";
import { type Payload, arrayContentEquals } from "@leftcurve/utils";

import type { AbciQueryResponse } from "@cosmjs/tendermint-rpc/build/comet38";
import type {
  AccountResponse,
  AccountStateResponse,
  Coin,
  InfoResponse,
  Proof,
  QueryRequest,
  QueryResponse,
  Tx,
} from "@leftcurve/types";

/**
 * Client for interacting with a Grug chain via Tendermint RPC.
 */
export class QueryClient {
  #cometClient: Comet38Client;

  /**
   * Do not use; use `Client.connect` instead.
   */
  protected constructor(cometClient: Comet38Client) {
    this.#cometClient = cometClient;
  }

  /**
   * Create a new Grug client for the given endpoint.
   *
   * Uses HTTP when the URL schema is http or https. Uses WebSockets otherwise.
   */
  public static async connect(endpoint: string | HttpEndpoint): Promise<QueryClient> {
    const cometClient = await Comet38Client.connect(endpoint);
    return new QueryClient(cometClient);
  }

  /**
   *
   * Createa a new Grug client given an RPC client.
   */
  public static async create(rpcClient: RpcClient): Promise<QueryClient> {
    const cometClient = await Comet38Client.create(rpcClient);
    return new QueryClient(cometClient);
  }

  // ------------------------------ query methods ------------------------------

  async #query(
    path: string,
    data: Uint8Array,
    height = 0,
    prove = false,
  ): Promise<AbciQueryResponse> {
    const res = await this.#cometClient.abciQuery({
      path,
      data,
      height,
      prove,
    });
    if (res.code !== 0) {
      throw new Error(
        `query failed! codespace: ${res.codespace}, code: ${res.code}, log: ${res.log}`,
      );
    }
    return res;
  }

  async queryStore(
    key: Uint8Array,
    height = 0,
    prove = false,
  ): Promise<{ value: Uint8Array | null; proof: Proof | null }> {
    const res = await this.#query("/store", key, height, prove);
    const value = res.value.length > 0 ? res.value : null;

    if (!prove) return { value, proof: null };

    if (!res.proof) {
      throw new Error("expected proof, got none");
    }

    const ops = res.proof.ops;
    // do some basic sanity checks on the proof op
    if (ops.length !== 1) {
      throw new Error(`expecting exactly one proof op, found ${ops.length}`);
    }
    if (ops[0].type !== "grug_jmt::Proof") {
      throw new Error(`unknown proof type: ${ops[0].type}`);
    }

    if (!arrayContentEquals(ops[0].key, key)) {
      throw new Error(
        `incorrect key! expecting: ${encodeBase64(key)}, found: ${encodeBase64(ops[0].key)}`,
      );
    }

    const proof = deserialize<Proof>(ops[0].data);

    return { value, proof };
  }

  async queryApp(req: QueryRequest, height = 0): Promise<QueryResponse> {
    const res = await this.#query("/app", serialize(req), height, false);
    return deserialize<QueryResponse>(res.value);
  }

  async queryWasmRaw(
    contract: string,
    key: Uint8Array,
    height = 0,
  ): Promise<Uint8Array | undefined> {
    const res = await this.queryApp(
      {
        wasmRaw: {
          contract,
          key: encodeBase64(key),
        },
      },
      height,
    );
    if (!("wasmRaw" in res)) {
      throw new Error(`expecting wasm raw response, got ${JSON.stringify(res)}`);
    }
    return res.wasmRaw.value !== undefined ? decodeBase64(res.wasmRaw.value) : undefined;
  }

  async queryWasmSmart<T>(contract: string, msg: Payload, height = 0): Promise<T> {
    const res = await this.queryApp(
      {
        wasmSmart: {
          contract,
          msg: encodeBase64(serialize(msg)),
        },
      },
      height,
    );
    if (!("wasmSmart" in res)) {
      throw new Error(`expecting wasm smart response, got ${JSON.stringify(res)}`);
    }
    return deserialize(decodeBase64(res.wasmSmart.data)) as T;
  }

  async getChainInfo(height = 0): Promise<InfoResponse> {
    const res = await this.queryApp(
      {
        info: {},
      },
      height,
    );
    if (!("info" in res)) {
      throw new Error(`expecting info response, got ${JSON.stringify(res)}`);
    }
    return res.info;
  }

  async getBalance(address: string, denom: string, height = 0): Promise<number> {
    const res = await this.queryApp(
      {
        balance: { address, denom },
      },
      height,
    );
    if (!("balance" in res)) {
      throw new Error(`expecting balance response, got ${JSON.stringify(res)}`);
    }
    return Number.parseInt(res.balance.amount);
  }

  async getBalances(
    address: string,
    startAfter?: string,
    limit?: number,
    height = 0,
  ): Promise<Coin[]> {
    const res = await this.queryApp(
      {
        balances: { address, startAfter, limit },
      },
      height,
    );
    if (!("balances" in res)) {
      throw new Error(`expecting balances response, got ${JSON.stringify(res)}`);
    }
    return res.balances;
  }

  public async getSupply(denom: string, height = 0): Promise<number> {
    const res = await this.queryApp(
      {
        supply: { denom },
      },
      height,
    );
    if (!("supply" in res)) {
      throw new Error(`expecting supply response, got ${JSON.stringify(res)}`);
    }
    return Number.parseInt(res.supply.amount);
  }

  async getSupplies(startAfter?: string, limit?: number, height = 0): Promise<Coin[]> {
    const res = await this.queryApp({ supplies: { startAfter, limit } }, height);
    if (!("supplies" in res)) {
      throw new Error(`expecting supplies response, got ${JSON.stringify(res)}`);
    }
    return res.supplies;
  }

  async getCode(hash: string, height = 0): Promise<Uint8Array> {
    const res = await this.queryApp(
      {
        code: { hash },
      },
      height,
    );
    if (!("code" in res)) {
      throw new Error(`expecting code response, got ${JSON.stringify(res)}`);
    }
    return decodeBase64(res.code);
  }

  async getCodes(startAfter?: string, limit?: number, height = 0): Promise<Uint8Array[]> {
    const res = await this.queryApp(
      {
        codes: { startAfter, limit },
      },
      height,
    );
    if (!("codes" in res)) {
      throw new Error(`expecting codes response, got ${JSON.stringify(res)}`);
    }
    return res.codes.map(decodeHex);
  }

  async getAccount(address: string, height = 0): Promise<AccountResponse> {
    const res = await this.queryApp(
      {
        account: { address },
      },
      height,
    );
    if (!("account" in res)) {
      throw new Error(`expecting account response, got ${JSON.stringify(res)}`);
    }
    return res.account;
  }

  async getAccounts(startAfter?: string, limit?: number, height = 0): Promise<AccountResponse[]> {
    const res = await this.queryApp(
      {
        accounts: { startAfter, limit },
      },
      height,
    );
    if (!("accounts" in res)) {
      throw new Error(`expecting accounts response, got ${JSON.stringify(res)}`);
    }
    return res.accounts;
  }

  async getPublicKey(keyId: string): Promise<unknown> {
    const publicKey = await this.queryWasmSmart(keyId, {
      unknown: {},
    });

    return publicKey;
  }

  async getAccountState(address: string, height = 0): Promise<AccountStateResponse> {
    const accountSate = await this.queryWasmSmart<AccountStateResponse>(
      address,
      { state: {} },
      height,
    );

    return accountSate;
  }

  public async broadcastTx(tx: Tx): Promise<Uint8Array> {
    const rawTx = serialize(tx);
    const { code, codespace, log, hash } = await this.#cometClient.broadcastTxSync({ tx: rawTx });

    if (code !== 0) {
      throw new Error(
        `failed to broadcast tx! codespace: ${codespace}, code: ${code}, log: ${log}`,
      );
    }

    return hash;
  }
}
