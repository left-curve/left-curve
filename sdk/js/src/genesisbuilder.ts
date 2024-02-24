import * as fs from "node:fs";
import * as os from "node:os";
import { sha256 } from "@cosmjs/crypto";
import {
  type AccountFactoryExecuteMsg,
  Addr,
  type AdminOption,
  Binary,
  type Config,
  Hash,
  type Message,
  type Payload,
  camelToSnake,
  createAdmin,
  deriveAddress,
  deriveSalt,
  encodeBase64,
  recursiveTransform,
  serialize,
} from ".";

export const GENESIS_SENDER = Addr.fromStr("0x0a367b92cf0b037dfd89960ee832d56f7fc151681bb41e53690e776f5786998a");
export const GENESIS_BLOCK_HASH = Hash.fromHex("d04b98f48e8f8bcc15c6ae5ac050801cd6dcfd428fb5f9e65c4e16e7807340fa");

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
  public storeCode(path: string): Hash {
    const wasmByteCode = fs.readFileSync(path);
    this.storeCodeMsgs.push({
      storeCode: {
        wasmByteCode: new Binary(wasmByteCode),
      },
    });
    return new Hash(sha256(wasmByteCode));
  }

  /**
   * Add an Instantiate message to genesis messages.
   * @returns The contract's address
   */
  public instantiate(
    codeHash: Hash,
    msg: Payload,
    salt: Uint8Array,
    adminOpt: AdminOption,
  ): Addr {
    this.otherMsgs.push({
      instantiate: {
        codeHash,
        msg: new Binary(serialize(msg)),
        salt: new Binary(salt),
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
  ): Addr {
    const codeHash = this.storeCode(path);
    return this.instantiate(codeHash, msg, salt, adminOpt);
  }

  /**
   * Add an Execute message to genesis messages.
   */
  public execute(contract: Addr, msg: Payload) {
    this.otherMsgs.push({
      execute: {
        contract,
        msg: new Binary(serialize(msg)),
        funds: [],
      },
    });
  }

  /**
   * Create an account using the account factory contract.
   * Note, we only support Secp256k1 keys now.
   */
  public registerAccount(factory: Addr, codeHash: Hash, secp256k1PublicKey: Uint8Array) {
    const serial = this.accountSerials.get(secp256k1PublicKey) ?? 0;
    const salt = deriveSalt("secp256k1", secp256k1PublicKey, serial);
    const address = deriveAddress(factory, codeHash, salt);
    const msg: AccountFactoryExecuteMsg = {
      registerAccount: {
        codeHash,
        publicKey: {
          secp256k1: encodeBase64(secp256k1PublicKey),
        },
      },
    };
    this.execute(factory, msg);
    this.accountSerials.set(secp256k1PublicKey, serial + 1);
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
