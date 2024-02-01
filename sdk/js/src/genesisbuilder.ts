import * as fs from "fs";
import * as os from "os";
import { Config, Message } from "./types";
import { Payload, encodeBase64, encodeHex, recursiveTransform, camelToSnake, serialize } from "./serde";
import { sha256 } from "@cosmjs/crypto";
import { AdminOption, createAdmin, deriveAddress } from "./client";

// during genesis, the zero address is used as the message sender.
const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000000000000000000000000000";

export class GenesisBuilder {
  storeCodeMsgs: Message[];
  otherMsgs: Message[];
  config?: Config;

  public constructor() {
    this.storeCodeMsgs = [];
    this.otherMsgs = [];
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
        admin: createAdmin(adminOpt, ZERO_ADDRESS, codeHash, salt),
      },
    });
    return deriveAddress(ZERO_ADDRESS, codeHash, salt);
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
    const wasmByteCode = fs.readFileSync(path);
    const codeHash = sha256(wasmByteCode);
    this.storeCodeMsgs.push({
      storeCode: {
        wasmByteCode: encodeBase64(wasmByteCode),
      },
    });
    this.otherMsgs.push({
      instantiate: {
        codeHash: encodeHex(codeHash),
        msg: btoa(serialize(msg)),
        salt: encodeBase64(salt),
        funds: [],
        admin: createAdmin(adminOpt, ZERO_ADDRESS, codeHash, salt),
      },
    });
    return deriveAddress(ZERO_ADDRESS, codeHash, salt);
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
