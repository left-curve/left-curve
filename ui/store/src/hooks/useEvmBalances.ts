import { useQuery } from "@tanstack/react-query";

import { chains } from "../hyperlane.js";
import { ERC20_ABI } from "@left-curve/dango/hyperlane";

import type { Chain } from "viem";
import type { Address } from "@left-curve/dango/types";

export type UseEvmBalancesParameters = {
  network: keyof typeof chains;
  address?: Address;
};

export function useEvmBalances(parameters: UseEvmBalancesParameters) {
  const { network, address } = parameters;

  return useQuery({
    enabled: !!address && !!network,
    queryKey: ["external-balances", network, address],
    queryFn: async () => {
      if (!address) return {};
      const { createPublicClient, http } = await import("viem");

      const evmClient = createPublicClient({
        chain: chains[network as keyof typeof chains] as Chain,
        transport: http(undefined, { batch: true }),
      });

      const nativeBalance = await evmClient.getBalance({ address });

      const { contracts } = chains[network as keyof typeof chains];

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
