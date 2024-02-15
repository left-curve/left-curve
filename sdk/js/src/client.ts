import { Sha256, sha256 } from "@cosmjs/crypto";
import { Comet38Client, HttpEndpoint } from "@cosmjs/tendermint-rpc";
import {
  decodeUtf8,
  deserialize,
  encodeBase64,
  encodeHex,
  encodeUtf8,
  serialize,
  Payload,
  decodeHex,
} from "./serde";
import {
  Account,
  AccountResponse,
  AccountStateResponse,
  Coin,
  Config,
  InfoResponse,
  Message,
  QueryRequest,
  QueryResponse,
} from "./types";
import { SigningKey } from "./signingkey";
import { AbciQueryResponse } from "@cosmjs/tendermint-rpc/build/comet38";

/**
 * Client for interacting with a CWD blockchain via Tendermint RPC.
 */
export class Client {
  cometClient: Comet38Client;

  /**
   * Do not use; use `Client.connect` instead.
   */
  private constructor(cometClient: Comet38Client) {
    this.cometClient = cometClient;
  }

  /**
   * Create a new CWD client for the given endpoint.
   *
   * Uses HTTP when the URL schema is http or https. Uses WebSockets otherwise.
   */
  public static async connect(endpoint: string | HttpEndpoint): Promise<Client> {
    const cometClient = await Comet38Client.connect(endpoint);
    return new Client(cometClient);
  }

  // ------------------------------ query methods ------------------------------

  private async query(
    path: string,
    data: Uint8Array,
    height = 0,
    prove = false,
  ): Promise<AbciQueryResponse> {
    const res = await this.cometClient.abciQuery({ path, data, height, prove });
    if (res.code !== 0) {
      throw new Error(`query failed! codespace: ${res.codespace}, code: ${res.code}, log: ${res.log}`);
    }
    return res;
  }

  public async queryStore(
    key: Uint8Array,
    height = 0,
    prove = false,
  ): Promise<Uint8Array | undefined> {
    const res = await this.query("/store", key, height, prove);
    const value = res.value.length > 0 ? res.value : undefined;
    // TODO: deserialize and return proof
    return value;
  }

  public async queryApp(req: QueryRequest, height = 0): Promise<QueryResponse> {
    const res = await this.query("/app", encodeUtf8(serialize(req)), height, false);
    return deserialize(decodeUtf8(res.value)) as QueryResponse;
  }

  public async queryInfo(height = 0): Promise<InfoResponse> {
    const res = await this.queryApp({ info: {} }, height);
    return res.info!;
  }

  public async queryBalance(address: string, denom: string, height = 0): Promise<string> {
    const res = await this.queryApp({ balance: { address, denom } }, height);
    return res.balance!.amount;
  }

  public async queryBalances(address: string, startAfter?: string, limit?: number, height = 0): Promise<Coin[]> {
    const res = await this.queryApp({ balances: { address, startAfter, limit } }, height);
    return res.balances!;
  }

  public async querySupply(denom: string, height = 0): Promise<string> {
    const res = await this.queryApp({ supply: { denom } }, height);
    return res.supply!.amount;
  }

  public async querySupplies(startAfter?: string, limit?: number, height = 0): Promise<Coin[]> {
    const res = await this.queryApp({ supplies: { startAfter, limit } }, height);
    return res.supplies!;
  }

  public async queryCode(hash: string, height = 0): Promise<string> {
    const res = await this.queryApp({ code: { hash } }, height);
    return res.code!;
  }

  public async queryCodes(startAfter?: string, limit?: number, height = 0): Promise<string[]> {
    const res = await this.queryApp({ codes: { startAfter, limit } }, height);
    return res.codes!;
  }

  public async queryAccount(address: string, height = 0): Promise<Account> {
    const res = await this.queryApp({ account: { address } }, height);
    const accountRes = res.account!;
    return {
      codeHash: accountRes.codeHash,
      admin: accountRes.admin,
    }
  }

  public async queryAccounts(startAfter?: string, limit?: number, height = 0): Promise<AccountResponse[]> {
    const res = await this.queryApp({ accounts: { startAfter, limit } }, height);
    return res.accounts!;
  }

  public async queryWasmRaw(contract: string, key: string, height = 0): Promise<string | undefined> {
    const res = await this.queryApp({ wasmRaw: { contract, key } }, height);
    return res.wasmRaw!.value;
  }

  public async queryWasmSmart<T>(contract: string, msg: Payload, height = 0): Promise<T> {
    const res = await this.queryApp({ wasmSmart: { contract, msg: btoa(serialize(msg)) } }, height);
    const wasmRes = deserialize(atob(res.wasmSmart!.data));
    return wasmRes as T;
  }

  // ------------------------------- tx methods --------------------------------

  public async sendTx(msgs: Message[], signOpts: SigningOptions): Promise<Uint8Array> {
    if (!signOpts.chainId) {
      const infoRes = await this.queryInfo();
      signOpts.chainId = infoRes.chainId;
    }

    if (!signOpts.sequence) {
      const accountStateRes: AccountStateResponse = await this.queryWasmSmart(signOpts.sender, { state: {} });
      signOpts.sequence = accountStateRes.sequence;
    }

    const tx = encodeUtf8(serialize(await signOpts.signingKey.createAndSignTx(
      msgs,
      signOpts.sender,
      signOpts.chainId,
      signOpts.sequence,
    )));

    const { code, codespace, log, hash } = await this.cometClient.broadcastTxSync({ tx });

    if (code === 0) {
      return hash;
    } else {
      throw new Error(`failed to broadcast tx! codespace: ${codespace}, code: ${code}, log: ${log}`);
    }
  }

  public async updateConfig(
    newCfg: Config,
    signOpts: SigningOptions,
  ): Promise<Uint8Array> {
    const updateCfgMsg = {
      updateConfig: { newCfg },
    };
    return this.sendTx([updateCfgMsg], signOpts);
  }

  public async transfer(
    to: string,
    coins: Coin[],
    signOpts: SigningOptions,
  ): Promise<Uint8Array> {
    const transferMsg = {
      transfer: { to, coins },
    };
    return this.sendTx([transferMsg], signOpts);
  }

  public async storeCode(
    wasmByteCode: Uint8Array,
    signOpts: SigningOptions,
  ): Promise<Uint8Array> {
    const storeCodeMsg = {
      storeCode: {
        wasmByteCode: encodeBase64(wasmByteCode),
      },
    };
    return this.sendTx([storeCodeMsg], signOpts);
  }

  public async instantiate(
    codeHash: Uint8Array,
    msg: Payload,
    salt: Uint8Array,
    funds: Coin[],
    adminOpt: AdminOption,
    signOpts: SigningOptions,
  ): Promise<[string, Uint8Array]> {
    const address = deriveAddress(signOpts.sender, codeHash, salt);
    const instantiateMsg = {
      instantiate: {
        codeHash: encodeHex(codeHash),
        msg: btoa(serialize(msg)),
        salt: encodeBase64(salt),
        funds,
        admin: createAdmin(adminOpt, signOpts.sender, codeHash, salt),
      },
    };
    const txhash = await this.sendTx([instantiateMsg], signOpts);
    return [address, txhash];
  }

  public async storeCodeAndInstantiate(
    wasmByteCode: Uint8Array,
    msg: Payload,
    salt: Uint8Array,
    funds: Coin[],
    adminOpt: AdminOption,
    signOpts: SigningOptions,
  ): Promise<[string, Uint8Array]> {
    const codeHash = sha256(wasmByteCode);
    const address = deriveAddress(signOpts.sender, codeHash, salt);
    const storeCodeMsg = {
      storeCode: {
        wasmByteCode: encodeBase64(wasmByteCode),
      },
    };
    const instantiateMsg = {
      instantiate: {
        codeHash: encodeHex(codeHash),
        msg: btoa(serialize(msg)),
        salt: encodeBase64(salt),
        funds,
        admin: createAdmin(adminOpt, signOpts.sender, codeHash, salt),
      },
    };
    const txhash = await this.sendTx([storeCodeMsg, instantiateMsg], signOpts);
    return [address, txhash];
  }

  public async execute(
    contract: string,
    msg: Payload,
    funds: Coin[],
    signOpts: SigningOptions,
  ): Promise<Uint8Array> {
    const executeMsg = {
      execute: {
        contract,
        msg: btoa(serialize(msg)),
        funds,
      },
    };
    return this.sendTx([executeMsg], signOpts);
  }

  public async migrate(
    contract: string,
    newCodeHash: Uint8Array,
    msg: Payload,
    signOpts: SigningOptions,
  ): Promise<Uint8Array> {
    const migrateMsg = {
      migrate: {
        contract,
        newCodeHash: encodeHex(newCodeHash),
        msg: btoa(serialize(msg)),
      },
    };
    return this.sendTx([migrateMsg], signOpts);
  }
}

export type SigningOptions = {
  signingKey: SigningKey;
  sender: string;
  chainId?: string;
  sequence?: number;
};

export enum AdminOptionKind {
  SetToSelf,
  SetToNone,
}

export type AdminOption = string | AdminOptionKind.SetToSelf | AdminOptionKind.SetToNone;

/**
 * Determine the admin address based on the given option.
 */
export function createAdmin(
  adminOpt: AdminOption,
  deployer: string,
  codeHash: Uint8Array,
  salt: Uint8Array,
): string | undefined {
  if (typeof adminOpt === "string") {
    return adminOpt;
  } else if (adminOpt === AdminOptionKind.SetToSelf) {
    return deriveAddress(deployer, codeHash, salt);
  } else {
    return undefined;
  }
}

/**
 * Derive an account address based on the deployer address, code hash, and salt.
 */
export function deriveAddress(deployer: string, codeHash: Uint8Array, salt: Uint8Array): string {
  const hasher = new Sha256();
  hasher.update(decodeHex(deployer.slice(2))); // note: remove the 0x prefix
  hasher.update(codeHash);
  hasher.update(salt);
  const bytes = hasher.digest();
  return "0x" + encodeHex(bytes);
}
