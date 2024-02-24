import { Sha256, sha256 } from "@cosmjs/crypto";
import { Comet38Client, type HttpEndpoint } from "@cosmjs/tendermint-rpc";
import type { AbciQueryResponse } from "@cosmjs/tendermint-rpc/build/comet38";
import {
  type AccountResponse,
  type AccountStateResponse,
  Addr,
  Binary,
  type Coin,
  type Config,
  Hash,
  type InfoResponse,
  type Message,
  type Payload,
  type QueryRequest,
  type QueryResponse,
  type SigningKey,
  type Uint,
  deserialize,
  encodeBigEndian32,
  encodeUtf8,
  serialize,
} from ".";

/**
 * Client for interacting with a CWD blockchain via Tendermint RPC.
 */
export class Client {
  private cometClient: Comet38Client;

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
    const res = await this.query("/app", serialize(req), height, false);
    return deserialize(res.value) as QueryResponse;
  }

  public async queryInfo(height = 0): Promise<InfoResponse> {
    const res = await this.queryApp(
      {
        info: {},
      },
      height,
    );
    return res.info!;
  }

  public async queryBalance(address: Addr, denom: string, height = 0): Promise<Uint> {
    const res = await this.queryApp(
      {
        balance: { address, denom },
      },
      height,
    );
    return res.balance!.amount;
  }

  public async queryBalances(
    address: Addr,
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
    return res.balances!;
  }

  public async querySupply(denom: string, height = 0): Promise<Uint> {
    const res = await this.queryApp(
      {
        supply: { denom },
      },
      height,
    );
    return res.supply!.amount;
  }

  public async querySupplies(startAfter?: string, limit?: number, height = 0): Promise<Coin[]> {
    const res = await this.queryApp(
      {
        supplies: { startAfter, limit },
      },
      height,
    );
    return res.supplies!;
  }

  public async queryCode(hash: Hash, height = 0): Promise<Binary> {
    const res = await this.queryApp(
      {
        code: { hash },
      },
      height,
    );
    return res.code!;
  }

  public async queryCodes(startAfter?: Hash, limit?: number, height = 0): Promise<Hash[]> {
    const res = await this.queryApp(
      {
        codes: { startAfter, limit },
      },
      height,
    );
    return res.codes!;
  }

  public async queryAccount(address: Addr, height = 0): Promise<AccountResponse> {
    const res = await this.queryApp(
      {
        account: { address },
      },
      height,
    );
    return res.account!;
  }

  public async queryAccounts(
    startAfter?: Addr,
    limit?: number,
    height = 0,
  ): Promise<AccountResponse[]> {
    const res = await this.queryApp(
      {
        accounts: { startAfter, limit },
      },
      height,
    );
    return res.accounts!;
  }

  public async queryWasmRaw(
    contract: Addr,
    key: Uint8Array,
    height = 0,
  ): Promise<Uint8Array | undefined> {
    const res = await this.queryApp(
      {
        wasmRaw: {
          contract,
          key: new Binary(key),
        },
      },
      height,
    );
    return res.wasmRaw!.value?.bytes;
  }

  public async queryWasmSmart<T>(contract: Addr, msg: Payload, height = 0): Promise<T> {
    const res = await this.queryApp(
      {
        wasmSmart: {
          contract,
          msg: new Binary(serialize(msg)),
        },
      },
      height,
    );
    return deserialize(res.wasmSmart!.data.bytes) as T;
  }

  // ------------------------------- tx methods --------------------------------

  public async sendTx(msgs: Message[], signOpts: SigningOptions): Promise<Uint8Array> {
    if (!signOpts.chainId) {
      const infoRes = await this.queryInfo();
      signOpts.chainId = infoRes.chainId;
    }

    if (!signOpts.sequence) {
      const accountStateRes: AccountStateResponse = await this.queryWasmSmart(
        signOpts.sender,
        {
          state: {},
        },
      );
      signOpts.sequence = accountStateRes.sequence;
    }

    const tx = serialize(await signOpts.signingKey.createAndSignTx(
      msgs,
      signOpts.sender,
      signOpts.chainId,
      signOpts.sequence,
    ));

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
    to: Addr,
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
        wasmByteCode: new Binary(wasmByteCode),
      },
    };
    return this.sendTx([storeCodeMsg], signOpts);
  }

  public async instantiate(
    codeHash: Hash,
    msg: Payload,
    salt: Uint8Array,
    funds: Coin[],
    adminOpt: AdminOption,
    signOpts: SigningOptions,
  ): Promise<[Addr, Uint8Array]> {
    const address = deriveAddress(signOpts.sender, codeHash, salt);
    const instantiateMsg = {
      instantiate: {
        codeHash,
        msg: new Binary(serialize(msg)),
        salt: new Binary(salt),
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
  ): Promise<[Addr, Uint8Array]> {
    const codeHash = new Hash(sha256(wasmByteCode));
    const address = deriveAddress(signOpts.sender, codeHash, salt);
    const storeCodeMsg = {
      storeCode: {
        wasmByteCode: new Binary(wasmByteCode),
      },
    };
    const instantiateMsg = {
      instantiate: {
        codeHash,
        msg: new Binary(serialize(msg)),
        salt: new Binary(salt),
        funds,
        admin: createAdmin(adminOpt, signOpts.sender, codeHash, salt),
      },
    };
    const txhash = await this.sendTx([storeCodeMsg, instantiateMsg], signOpts);
    return [address, txhash];
  }

  public async execute(
    contract: Addr,
    msg: Payload,
    funds: Coin[],
    signOpts: SigningOptions,
  ): Promise<Uint8Array> {
    const executeMsg = {
      execute: {
        contract,
        msg: new Binary(serialize(msg)),
        funds,
      },
    };
    return this.sendTx([executeMsg], signOpts);
  }

  public async migrate(
    contract: Addr,
    newCodeHash: Hash,
    msg: Payload,
    signOpts: SigningOptions,
  ): Promise<Uint8Array> {
    const migrateMsg = {
      migrate: {
        contract,
        newCodeHash,
        msg: new Binary(serialize(msg)),
      },
    };
    return this.sendTx([migrateMsg], signOpts);
  }
}

export type SigningOptions = {
  signingKey: SigningKey;
  sender: Addr;
  chainId?: string;
  sequence?: number;
};

export enum AdminOptionKind {
  SetToSelf,
  SetToNone,
}

export type AdminOption = Addr | AdminOptionKind.SetToSelf | AdminOptionKind.SetToNone;

/**
 * Determine the admin address based on the given option.
 */
export function createAdmin(
  adminOpt: AdminOption,
  deployer: Addr,
  codeHash: Hash,
  salt: Uint8Array,
): Addr | undefined {
  if (adminOpt instanceof Addr) {
    return adminOpt;
  } else if (adminOpt === AdminOptionKind.SetToSelf) {
    return deriveAddress(deployer, codeHash, salt);
  } else {
    return undefined;
  }
}

/**
 * Derive the salt that is used by the standard account factory contract to
 * register accounts.
 *
 * Mirrors the Rust function: `cw_account_factory::make_salt`.
 */
export function deriveSalt(
  publicKeyType: "secp256k1" | "secp256r1",
  publicKeyBytes: Uint8Array,
  serial: number,
): Uint8Array {
  const hasher = new Sha256();
  hasher.update(encodeUtf8(publicKeyType));
  hasher.update(publicKeyBytes);
  hasher.update(encodeBigEndian32(serial));
  return hasher.digest();
}

/**
 * Derive an account address based on the deployer address, code hash, and salt.
 *
 * Mirrors that Rust function: `cw_std::Addr::compute`
 */
export function deriveAddress(deployer: Addr, codeHash: Hash, salt: Uint8Array): Addr {
  const hasher = new Sha256();
  hasher.update(deployer.bytes);
  hasher.update(codeHash.bytes);
  hasher.update(salt);
  const bytes = hasher.digest();
  return new Addr(bytes);
}
