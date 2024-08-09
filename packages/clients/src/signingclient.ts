import type { HttpEndpoint, RpcClient } from "@cosmjs/tendermint-rpc";
import { sha256 } from "@leftcurve/crypto";
import { encodeBase64, encodeHex, serialize } from "@leftcurve/encoding";
import { createAddress } from "@leftcurve/types";
import { QueryClient } from "./queryclient";

import type { AbstractSigner, ChainConfig, Coin, Message } from "@leftcurve/types";
import type { Payload } from "@leftcurve/utils";

type ClientConfig = {
  chainId: string;
};

/**
 * Client for interacting with a Grug chain via Tendermint RPC.
 */
export class SigningClient {
  #query: QueryClient;
  #signer: AbstractSigner;
  #chainId: string;

  /**
   * Do not use; use `Client.connect` instead.
   */
  protected constructor(
    queryClient: QueryClient,
    signer: AbstractSigner,
    { chainId }: ClientConfig,
  ) {
    this.#query = queryClient;
    this.#signer = signer;
    this.#chainId = chainId;
  }

  /**
   * Create a new Grug client for the given endpoint.
   *
   * Uses HTTP when the URL schema is http or https. Uses WebSockets otherwise.
   */
  public static async connectWithSigner(
    endpoint: string | HttpEndpoint,
    signer: AbstractSigner,
  ): Promise<SigningClient> {
    const queryClient = await QueryClient.connect(endpoint);
    const { chainId } = await queryClient.getChainInfo();
    return new SigningClient(queryClient, signer, { chainId });
  }

  /**
   *
   * Createa a new Grug client given an RPC client.
   */
  public static async createWithSigner(
    rpcClient: RpcClient,
    signer: AbstractSigner,
  ): Promise<SigningClient> {
    const queryClient = await QueryClient.create(rpcClient);
    const { chainId } = await queryClient.getChainInfo();
    return new SigningClient(queryClient, signer, { chainId });
  }

  async signAndBroadcastTx(sender: string, msgs: Message[]): Promise<Uint8Array> {
    const accountState = await this.#query.getAccountState(sender).catch(() => undefined);

    const tx = await this.#signer.signTx(msgs, sender, this.#chainId, accountState);

    return this.#query.broadcastTx(tx);
  }

  public async updateConfig(sender: string, newCfg: ChainConfig): Promise<Uint8Array> {
    const updateCfgMsg = {
      updateConfig: { newCfg },
    };

    return this.signAndBroadcastTx(sender, [updateCfgMsg]);
  }

  public async transfer(sender: string, to: string, coins: Coin[]): Promise<Uint8Array> {
    const transferMsg = { transfer: { to, coins } };
    return this.signAndBroadcastTx(sender, [transferMsg]);
  }

  public async storeCode(sender: string, wasmByteCode: Uint8Array): Promise<Uint8Array> {
    const storeCodeMsg = {
      storeCode: {
        wasmByteCode: encodeBase64(wasmByteCode),
      },
    };
    return this.signAndBroadcastTx(sender, [storeCodeMsg]);
  }

  public async instantiate(
    sender: string,
    codeHash: Uint8Array,
    msg: Payload,
    salt: Uint8Array,
    funds: Coin[],
    adminOpt?: AdminOption,
  ): Promise<[string, Uint8Array]> {
    const address = createAddress(sender, codeHash, salt);
    const instantiateMsg = {
      instantiate: {
        codeHash: encodeHex(codeHash),
        msg: encodeBase64(serialize(msg)),
        salt: encodeBase64(salt),
        funds,
        admin: determineAdmin(adminOpt, sender, codeHash, salt),
      },
    };
    const txhash = await this.signAndBroadcastTx(sender, [instantiateMsg]);
    return [address, txhash];
  }

  public async storeCodeAndInstantiate(
    sender: string,
    wasmByteCode: Uint8Array,
    msg: Payload,
    salt: Uint8Array,
    funds: Coin[],
    adminOpt?: AdminOption,
  ): Promise<[string, Uint8Array]> {
    const codeHash = sha256(wasmByteCode);
    const address = createAddress(sender, codeHash, salt);
    const storeCodeMsg = {
      storeCode: {
        wasmByteCode: encodeBase64(wasmByteCode),
      },
    };
    const instantiateMsg = {
      instantiate: {
        codeHash: encodeHex(codeHash),
        msg: encodeBase64(serialize(msg)),
        salt: encodeBase64(salt),
        funds,
        admin: determineAdmin(adminOpt, sender, codeHash, salt),
      },
    };
    const txhash = await this.signAndBroadcastTx(sender, [storeCodeMsg, instantiateMsg]);
    return [address, txhash];
  }

  public async execute(
    sender: string,
    contract: string,
    msg: Payload,
    funds: Coin[],
  ): Promise<Uint8Array> {
    const executeMsg = {
      execute: {
        contract,
        msg: encodeBase64(serialize(msg)),
        funds,
      },
    };
    return this.signAndBroadcastTx(sender, [executeMsg]);
  }

  async migrate(
    sender: string,
    contract: string,
    newCodeHash: Uint8Array,
    msg: Payload,
  ): Promise<Uint8Array> {
    const migrateMsg = {
      migrate: {
        contract,
        newCodeHash: encodeHex(newCodeHash),
        msg: encodeBase64(serialize(msg)),
      },
    };
    return this.signAndBroadcastTx(sender, [migrateMsg]);
  }
}

export enum AdminOptionKind {
  SetToSelf = 0,
  SetToNone = 1,
}

export type AdminOption =
  | string
  | AdminOptionKind.SetToSelf
  | AdminOptionKind.SetToNone
  | undefined;

/**
 * Determine the admin address based on the given option.
 */
export function determineAdmin(
  adminOpt: AdminOption,
  deployer: string,
  codeHash: Uint8Array,
  salt: Uint8Array,
): string | undefined {
  if (adminOpt === AdminOptionKind.SetToSelf) {
    return createAddress(deployer, codeHash, salt);
  }
  if (adminOpt === AdminOptionKind.SetToNone) {
    return undefined;
  }
  return adminOpt;
}
