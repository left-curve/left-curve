import { useQuery } from "@tanstack/react-query";

import { ERC20_ABI } from "@left-curve/dango/hyperlane";

import type { Chain } from "viem";
import type { Address } from "@left-curve/dango/types";
import type { useBridgeState } from "./useBridgeState.js";

export type UseEvmBalancesParameters = {
  chain: NonNullable<ReturnType<typeof useBridgeState>["config"]>["chain"];
  address?: Address;
};

const infuraUrl = {
  "1": "https://mainnet.infura.io/v3/00f81bbb13ef4da997f6351b8146807e",
  "11155111": "https://sepolia.infura.io/v3/2de96f6db6d34eccaa8935cabb9b29c8",
  "8453": "base",
  "42161": "arbitrum",
};

export function useEvmBalances(parameters: UseEvmBalancesParameters) {
  const { chain, address } = parameters;

  return useQuery({
    enabled: !!address,
    queryKey: ["external-balances", chain, address],
    queryFn: async () => {
      if (!address) return {};
      const { createPublicClient, http } = await import("viem");

      const evmClient = createPublicClient({
        chain: chain as Chain,
        transport: http(infuraUrl[chain.id], {
          batch: true,
        }),
      });

      const nativeBalance = await evmClient.getBalance({ address });

      const { contracts } = chain;

      const erc20Balances = await Promise.all(
        contracts.erc20.map(async (token) => {
          const balance = await evmClient.readContract({
            address: token.address as `0x${string}`,
            abi: ERC20_ABI,
            functionName: "balanceOf",
            args: [address as `0x${string}`],
          });

          return { [token.targetDenom]: balance.toString() };
        }),
      );

      return Object.assign({ "bridge/eth": nativeBalance.toString() }, ...erc20Balances);
    },
  });
}
