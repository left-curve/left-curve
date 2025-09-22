import {
  transfer,
  type GetAccountsByUsernameParameters,
  type GetBalancesParameters,
} from "@left-curve/dango/actions";
import type { Account, Coin, Denom, PublicClient, SignerClient } from "@left-curve/dango/types";
import { formatUnits, parseUnits } from "@left-curve/dango/utils";
import type { AnyCoin } from "@left-curve/store/types";

import type { ChatCompletionTool, MLCEngine } from "@mlc-ai/web-llm";

export function createDangoTools(
  _engine: MLCEngine,
  client: PublicClient | SignerClient,
  account: Account | null,
  coins: Record<Denom, AnyCoin>,
): { definition: ChatCompletionTool; fn: <P, R>(params: P) => Promise<R> }[] {
  return [
    // getBalances tool
    {
      definition: {
        type: "function",
        function: {
          name: "getBalances",
          description: "Get the balances for a given address",
          parameters: {
            type: "object",
            properties: {
              address: {
                type: "string",
                description:
                  "An account address in Dango, which is a 20-byte length, in Hex encoding with a `0x` prefix.",
              },
            },
            required: ["address"],
          },
        },
      },
      fn: async (params: GetBalancesParameters) => {
        const { getBalances } = await import("@left-curve/dango/actions");
        const balances = await getBalances(client, params);
        return Object.fromEntries(
          Object.entries(balances).map(([denom, amount]) => {
            const coin = coins[denom as Denom];
            if (!coin) return [denom, amount];
            return [denom, formatUnits(amount, coin.decimals)];
          }),
        );
      },
    },
    // get accounts by username tool
    {
      definition: {
        type: "function",
        function: {
          name: "getAccountsByUsername",
          description: "Get the accounts for a given username",
          parameters: {
            type: "object",
            properties: {
              username: {
                type: "string",
                description:
                  "A name that uniquely identifies a user. consisting of lowercase letters, numbers, or underscores, between 1-15 characters.",
              },
            },
            required: ["username"],
          },
        },
      },
      fn: async (params: GetAccountsByUsernameParameters) => {
        const { getAccountsByUsername } = await import("@left-curve/dango/actions");
        return await getAccountsByUsername(client, params);
      },
    },
    // transfer tool
    {
      definition: {
        type: "function",
        function: {
          name: "transfer",
          description: "Transfer coins to a given address",
          parameters: {
            type: "object",
            properties: {
              address: {
                type: "string",
                description:
                  "An account address in Dango, which is a 40 character length, in Hex encoding with a `0x` prefix.",
              },
              coin: {
                type: "object",
                properties: {
                  denom: {
                    type: "string",
                    description: "Coin denom (e.g. 'dango', 'bridge/btc') from getCoins tool",
                  },
                  amount: {
                    type: "string",
                    description: "The coin amount as string",
                  },
                },
                required: ["denom", "amount"],
              },
            },
            required: ["address", "coin"],
          },
        },
      },
      fn: async (params: { address: string; coin: Coin }) => {
        if (!account) return "User need to be logged in to use this tool";
        const coin = coins[params.coin.denom as Denom];
        return await transfer(client as SignerClient, {
          sender: account.address,
          transfer: {
            [params.address]: {
              [params.coin.denom]: parseUnits(params.coin.amount, coin.decimals),
            },
          },
        });
      },
    },
    // get coins
    {
      definition: {
        type: "function",
        function: {
          name: "getCoins",
          description: "Get the list of supported coins in Dango",
          parameters: {
            type: "object",
            properties: {},
          },
        },
      },
      fn: async () => {
        return Object.values(coins);
      },
    },
  ] as any;
}
