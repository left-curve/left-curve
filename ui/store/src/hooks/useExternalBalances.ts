import { useQuery } from "@tanstack/react-query";

import { chains } from "../hyperlane.js";
import { ERC20_ABI } from "@left-curve/dango/hyperlane";

import type { Chain } from "viem";
import type { Address } from "@left-curve/dango/types";

type ExternalNetworks = "bitcoin" | "ethereum" | "arbitrum" | "base" | "solana";

export type UseExternalBalancesParameters = {
  network: ExternalNetworks | null;
  address?: Address;
};

const evmChains = ["ethereum", "arbitrum", "base"];

export function useExternalBalances(parameters: UseExternalBalancesParameters) {
  const { network, address } = parameters;

  const { data: evmClient } = useQuery({
    enabled: evmChains.includes(network as string),
    queryKey: ["evmClient", network],
    queryFn: async () => {
      if (!evmChains.includes(network as string)) return null;
      const { createPublicClient, http } = await import("viem");
      return createPublicClient({
        chain: chains[network as keyof typeof chains] as Chain,
        transport: http(undefined, { batch: true }),
      });
    },
  });

  return useQuery({
    enabled: !!address && !!network,
    queryKey: ["external-balances", network, address],
    queryFn: async () => {
      if (!address || !network) return {};
      if (network === "bitcoin") return {};
      if (network === "solana") return {};

      if (!evmClient) return {};

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
