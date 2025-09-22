import type {
  GetAccountsByUsernameParameters,
  GetBalancesParameters,
} from "@left-curve/dango/actions";
import type { PublicClient, SignerClient } from "@left-curve/dango/types";

export function createDangoTools(client: PublicClient | SignerClient) {
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
                description: "The address to get balances for",
              },
              startAfter: {
                type: "string",
                description: "The denom to start after",
              },
              limit: {
                type: "number",
                description: "The maximum number of balances to return",
              },
            },
            required: ["address"],
          },
        },
      },
      getBalances: async (params: GetBalancesParameters) => {
        const { getBalances } = await import("@left-curve/dango/actions");
        return await getBalances(client, params);
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
                description: "The username to get accounts for",
              },
            },
            required: ["username"],
          },
        },
      },
      getAccountsByUsername: async (params: GetAccountsByUsernameParameters) => {
        const { getAccountsByUsername } = await import("@left-curve/dango/actions");
        return await getAccountsByUsername(client, params);
      },
    },
  ];
}
