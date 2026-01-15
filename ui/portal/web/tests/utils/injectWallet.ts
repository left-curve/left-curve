import type { Page } from "@playwright/test";
import { getEip6963MockWalletScript, type MockWalletOptions } from "../mocks/eip6963";
import { privateKeyToAccount } from "viem/accounts";
import type { Hex } from "viem";

export interface WalletInjectionOptions extends MockWalletOptions {
  privateKey?: Hex;
}

export const injectMockWallet = async (page: Page, options: WalletInjectionOptions = {}) => {
  let accounts = options.accounts;
  let privateKey = options.privateKey;

  // Default private key if none provided and no accounts (Hardhat/Anvil account #0)
  if (!privateKey && !accounts) {
    privateKey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
  }

  // Derive account if we have a key
  if (privateKey) {
    const account = privateKeyToAccount(privateKey);
    if (!accounts) {
      accounts = [account.address];
    }

    // Expose the signing function
    await page.exposeFunction("mockWalletSign", async (method: string, params: any[]) => {
      if (method === "personal_sign") {
        // params: [message, address]
        const [message] = params;
        // message is expected to be a hex string from the dapp
        return await account.signMessage({ message: { raw: message as Hex } });
      }
      if (method === "eth_sign") {
        // params: [address, message]
        const [_, message] = params;
        return await account.signMessage({ message: { raw: message as Hex } });
      }
      throw new Error(`mockWalletSign: Unsupported method ${method}`);
    });
  }

  const script = getEip6963MockWalletScript({
    ...options,
    accounts,
  });
  await page.addInitScript({ content: script });
};
