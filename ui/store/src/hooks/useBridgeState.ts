import { useCallback, useEffect, useMemo, useState } from "react";
import { useConnectors } from "./useConnectors.js";
import { useSubmitTx } from "./useSubmitTx.js";
import { useQuery } from "@tanstack/react-query";
import { useConfig } from "./useConfig.js";
import { useAccount } from "./useAccount.js";
import { useSigningClient } from "./useSigningClient.js";
import { useExternalBalances, type UseExternalBalancesParameters } from "./useExternalBalances.js";

import hyperlaneConfig from "../../../../dango/hyperlane-deployment/config.json" with {
  type: "json",
};

import { chains } from "../hyperlane.js";
import { ERC20_ABI, HYPERLANE_ROUTER_ABI, toAddr32 } from "@left-curve/dango/hyperlane";
import { createPublicClient, createWalletClient, custom, http } from "viem";
import { parseUnits } from "@left-curve/dango/utils";
import { transferRemote } from "@left-curve/dango/actions";

import type { EIP1193Provider } from "../types/eip1193.js";
import type { MailBoxConfig } from "@left-curve/dango/types";
import type { AnyCoin } from "../types/coin.js";
import type { Chain as ViemChain } from "viem";

export type UseBridgeStateParameters = {
  action: "deposit" | "withdraw";
  controllers: {
    inputs: Record<string, { value: string }>;
    reset: () => void;
    setValue: (name: string, value: string) => void;
  };
};

export function useBridgeState(params: UseBridgeStateParameters) {
  const { action, controllers } = params;
  const { account } = useAccount();
  const { coins: allCoins, getAppConfig } = useConfig();
  const [coin, setCoin] = useState<AnyCoin | null>(null);
  const [network, setNetwork] = useState<string | null>(null);
  const [connectorId, setConnectorId] = useState<string | null>(null);
  const [getAmount, setGetAmount] = useState<string>("0");
  const connectors = useConnectors();
  const { data: signingClient } = useSigningClient();
  const { inputs } = controllers;

  const operationAmount = inputs.amount?.value || "0";

  const changeCoin = useCallback((denom: string) => setCoin(allCoins.byDenom[denom]), [allCoins]);

  const connector = useMemo(
    () => connectors.find((c) => c.id === connectorId),
    [connectorId, connectors],
  );

  const walletAddress = useQuery({
    enabled: action === "deposit" && !!connector,
    queryKey: ["bridge", "connectedAddress", connectorId],
    queryFn: async () => {
      const provider = await (
        connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
      ).getProvider();
      const [account] = await provider.request({ method: "eth_requestAccounts" });
      return account;
    },
  });

  const { data: externalBalances = {} } = useExternalBalances({
    network: network as UseExternalBalancesParameters["network"],
    address: walletAddress?.data,
  });

  const coins = useMemo(() => {
    return Object.values(allCoins.byDenom).filter((c) =>
      ["USDC", "ETH", "USDT"].includes(c.symbol),
    );
  }, [allCoins]);

  const deposit = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!coin) throw new Error("Coin not selected");
        if (!account) throw new Error("Account not connected");

        const originChain = chains[network as keyof typeof chains] as ViemChain;
        if (!originChain) throw new Error(`Chain ${network} not configured`);

        const originConfig = hyperlaneConfig.evm[network as keyof typeof hyperlaneConfig.evm];
        if (!originConfig) throw new Error(`Hyperlane config not found for ${network}`);

        const routeConfig = originConfig.warp_routes.find((r) =>
          r.symbol.toLowerCase().includes(coin.symbol.toLowerCase()),
        );

        if (!routeConfig) throw new Error(`Warp route not found for ${coin.symbol} on ${network}`);

        const appConfig = await getAppConfig();
        const mailboxConfig: MailBoxConfig | undefined = await signingClient?.queryWasmSmart({
          contract: appConfig.addresses.mailbox,
          msg: { config: {} },
        });

        if (!mailboxConfig) throw new Error("Mailbox config not found");

        const provider = await (
          connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
        ).getProvider();

        const walletClient = createWalletClient({
          chain: originChain,
          transport: custom(provider),
        });

        await walletClient.switchChain({ id: originChain.id });

        const publicClient = createPublicClient({
          chain: originChain,
          transport: http(),
        });

        const [evmAddress] = await walletClient.requestAddresses();

        const amount = BigInt(parseUnits(operationAmount, coin.decimals));
        const destinationDomain = mailboxConfig.localDomain;
        const protocolFee = BigInt(originConfig.hyperlane_protocol_fee);
        const routerAddress = routeConfig.proxy_address as `0x${string}`;
        const recipientAddress = toAddr32(account.address);

        const value = await (async () => {
          if (typeof routeConfig.warp_route_type !== "string") {
            const tokenAddress = routeConfig.warp_route_type.erc20_collateral as `0x${string}`;

            const allowance = await publicClient.readContract({
              address: tokenAddress,
              abi: ERC20_ABI,
              functionName: "allowance",
              args: [evmAddress, routerAddress],
            });

            if (allowance < amount) {
              const approveHash = await walletClient.writeContract({
                address: tokenAddress,
                abi: ERC20_ABI,
                functionName: "approve",
                args: [routerAddress, amount],
                account: evmAddress,
              });

              await publicClient.waitForTransactionReceipt({ hash: approveHash });
            }
            return protocolFee;
          }

          return amount + protocolFee;
        })();

        const txHash = await walletClient.writeContract({
          address: routerAddress,
          abi: HYPERLANE_ROUTER_ABI,
          functionName: "transferRemote",
          args: [destinationDomain, recipientAddress, amount],
          value,
          account: evmAddress,
        });

        await publicClient.waitForTransactionReceipt({ hash: txHash });
      },
    },
  });

  const withdraw = useSubmitTx({
    mutation: {
      mutationFn: async () => {
        if (!signingClient) throw new Error("Signing client not initialized");
        if (!account) throw new Error("Account not connected");
        if (!coin) throw new Error("Coin not selected");

        const targetChain = chains[network as keyof typeof chains] as ViemChain;
        if (!targetChain) throw new Error(`Chain ${network} not configured`);

        const targetConfig = hyperlaneConfig.evm[network as keyof typeof hyperlaneConfig.evm];
        if (!targetConfig) throw new Error(`Hyperlane config not found for ${network}`);

        const routeConfig = targetConfig.warp_routes.find((r) =>
          r.symbol.toLowerCase().includes(coin.symbol.toLowerCase()),
        );

        if (!routeConfig) throw new Error(`Warp route not found for ${coin.symbol} on ${network}`);

        await transferRemote(signingClient, {
          sender: account.address,
          recipient: toAddr32(account.address),
          remote: {
            warp: {
              domain: targetConfig.hyperlane_domain,
              contract: toAddr32(routeConfig.proxy_address as `0x${string}`),
            },
          },
        });
      },
    },
  });

  const { data: depositAddress } = useQuery({
    queryKey: ["bridge", "depositAddress", network],
    queryFn: async () => {
      if (!["bitcoin"].includes(network as string)) return null;
      return "address";
    },
  });

  useEffect(() => {
    setCoin(null);
    setNetwork(null);
    setConnectorId(null);
    controllers.reset();
  }, [action]);

  useEffect(() => {
    setGetAmount(operationAmount);
  }, [operationAmount]);

  return {
    action,
    coins,
    coin,
    changeCoin,
    network,
    setNetwork,
    connector,
    setConnectorId,
    withdraw,
    deposit,
    depositAddress,
    walletAddress,
    getAmount,
    externalBalances,
  };
}
