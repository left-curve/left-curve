import * as fs from "node:fs";
import * as os from "node:os";
import type { AccountFactoryExecuteMsg, Config, Message } from "./types";
import {
  type Payload,
  decodeHex,
  encodeBase64,
  encodeHex,
  recursiveTransform,
  camelToSnake,
  serialize,
} from "./serde";
import { sha256 } from "@cosmjs/crypto";
import { type AdminOption, createAdmin, deriveAddress, deriveSalt } from "./client";

export const GENESIS_SENDER = "0x0a367b92cf0b037dfd89960ee832d56f7fc151681bb41e53690e776f5786998a";
export const GENESIS_BLOCK_HASH = decodeHex("d04b98f48e8f8bcc15c6ae5ac050801cd6dcfd428fb5f9e65c4e16e7807340fa");

export class GenesisBuilder {
  storeCodeMsgs: Message[];
  otherMsgs: Message[];
  accountSerials: Map<Uint8Array, number>;
  config?: Config;

  public constructor() {
    this.storeCodeMsgs = [];
    this.otherMsgs = [];
    this.accountSerials = new Map();
  }

  /**
   * Add a StoreCode message to genesis messages.
   * @param path Path to the Wasm binary file
   * @returns The code's SHA-256 hash.
   */
  public storeCode(path: string): Uint8Array {
    const wasmByteCode = fs.readFileSync(path);
    this.storeCodeMsgs.push({
      storeCode: {
        wasmByteCode: encodeBase64(wasmByteCode),
      },
    });
    return sha256(wasmByteCode);
  }

  /**
   * Add an Instantiate message to genesis messages.
   * @returns The contract's address
   */
  public instantiate(
    codeHash: Uint8Array,
    msg: Payload,
    salt: Uint8Array,
    adminOpt: AdminOption,
  ): string {
    this.otherMsgs.push({
      instantiate: {
        codeHash: encodeHex(codeHash),
        msg: btoa(serialize(msg)),
        salt: encodeBase64(salt),
        funds: [],
        admin: createAdmin(adminOpt, GENESIS_SENDER, codeHash, salt),
      },
    });
    return deriveAddress(GENESIS_SENDER, codeHash, salt);
  }

  /**
   * Add a StoreCode and an Instantitae message to genesis message in one go.
   * @returns The contract's address
   */
  public storeCodeAndInstantiate(
    path: string,
    msg: Payload,
    salt: Uint8Array,
    adminOpt: AdminOption,
  ): string {
    const codeHash = this.storeCode(path);
    return this.instantiate(codeHash, msg, salt, adminOpt);
  }

  /**
   * Add an Execute message to genesis messages.
   */
  public execute(contract: string, msg: Payload) {
    this.otherMsgs.push({
      execute: {
        contract,
        msg: btoa(serialize(msg)),
        funds: [],
      },
    });
  }

  /**
   * Create an account using the account factory contract.
   * Note, we only support Secp256k1 keys now.
   */
  public registerAccount(factory: string, codeHash: Uint8Array, publicKey: Uint8Array) {
    const serial = this.accountSerials.get(publicKey) ?? 0;
    const salt = deriveSalt("secp256k1", publicKey, serial);
    const address = deriveAddress(factory, codeHash, salt);
    const msg: AccountFactoryExecuteMsg = {
      registerAccount: {
        codeHash: encodeBase64(codeHash),
        publicKey: {
          secp256k1: encodeBase64(publicKey),
        },
      },
    };
    this.execute(factory, msg);
    this.accountSerials.set(publicKey, serial + 1);
    return address;
  }

  /**
   * Set the chain config.
   */
  public setConfig(config: Config) {
    if (this.config) {
      throw new Error("config is already set");
    }
    this.config = config;
  }

  /**
   * Write the genesis state to Tendermint genesis file.
   */
  public writeToFile(cometGenPath = `${os.homedir()}/.cometbft/config/genesis.json`) {
    if (!this.config) {
      throw new Error("config is not set");
    }
    const cometGen = JSON.parse(fs.readFileSync(cometGenPath, "utf8")) as { [key: string]: any };
    const appState = {
      config: this.config,
      msgs: [...this.storeCodeMsgs, ...this.otherMsgs],
    };
    cometGen["app_state"] = recursiveTransform(appState, camelToSnake);
    fs.writeFileSync(cometGenPath, JSON.stringify(cometGen, null, 2) + "\n");
  }
}
