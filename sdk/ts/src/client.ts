import { Comet38Client, HttpEndpoint } from "@cosmjs/tendermint-rpc";
import { Message, decodeUtf8, deserialize, encodeUtf8, serialize } from "./serde";
import { Account, AccountResponse, Coin, InfoResponse, QueryRequest, QueryResponse } from "./types";

export class Client {
  inner: Comet38Client;

  /**
   * Do not use; use `Client.connect` instead.
   */
  private constructor(inner: Comet38Client) {
    this.inner = inner;
  }

  /**
   * Create a new CWD client for the given endpoint.
   */
  public static async connect(endpoint: string | HttpEndpoint): Promise<Client> {
    const inner = await Comet38Client.connect(endpoint);
    return new Client(inner);
  }

  // ------------------------------ query methods ------------------------------

  private async query(req: QueryRequest): Promise<QueryResponse> {
    const res = await this.inner.abciQuery({
      path: "",
      data: encodeUtf8(serialize(req)),
    });

    if (res.code !== 0) {
      throw new Error(`query failed! codespace: ${res.codespace}, code: ${res.code}`);
    }

    return deserialize(decodeUtf8(res.value)) as QueryResponse;
  }

  public async queryInfo(): Promise<InfoResponse> {
    const res = await this.query({ info: {} });
    return res.info!;
  }

  public async queryBalance(address: string, denom: string): Promise<string> {
    const res = await this.query({ balance: { address, denom } });
    return res.balance!.amount;
  }

  public async queryBalances(address: string, startAfter?: string, limit?: number): Promise<Coin[]> {
    const res = await this.query({ balances: { address, startAfter, limit } });
    return res.balances!;
  }

  public async querySupply(denom: string): Promise<string> {
    const res = await this.query({ supply: { denom } });
    return res.supply!.amount;
  }

  public async querySupplies(startAfter?: string, limit?: number): Promise<Coin[]> {
    const res = await this.query({ supplies: { startAfter, limit } });
    return res.supplies!;
  }

  public async queryCode(hash: string): Promise<string> {
    const res = await this.query({ code: { hash } });
    return res.code!;
  }

  public async queryCodes(startAfter?: string, limit?: number): Promise<string[]> {
    const res = await this.query({ codes: { startAfter, limit } });
    return res.codes!;
  }

  public async queryAccount(address: string): Promise<Account> {
    const res = await this.query({ account: { address } });
    const accountRes = res.account!;
    return {
      codeHash: accountRes.codeHash,
      admin: accountRes.admin,
    }
  }

  public async queryAccounts(startAfter?: string, limit?: number): Promise<AccountResponse[]> {
    const res = await this.query({ accounts: { startAfter, limit } });
    return res.accounts!;
  }

  public async queryWasmRaw(contract: string, key: string): Promise<string | undefined> {
    const res = await this.query({ wasmRaw: { contract, key } });
    return res.wasmRaw!.value;
  }

  public async queryWasmSmart<T>(contract: string, msg: Message): Promise<T> {
    const res = await this.query({ wasmSmart: { contract, msg: btoa(serialize(msg)) } });
    const wasmRes = deserialize(atob(res.wasmSmart!.data));
    return wasmRes as T;
  }

  // ------------------------------- tx methods --------------------------------

  // TODO.........
}
