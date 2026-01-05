import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useConnectors } from "./useConnectors.js";
import { useConfig } from "./useConfig.js";
import { useStorage } from "./useStorage.js";

import { chains } from "../hyperlane.js";
import { toAddr32 } from "@left-curve/dango/hyperlane";

import type { AnyCoin } from "../types/coin.js";
import type { HyperlaneConfig } from "@left-curve/dango/types";

export type UseBridgeStateParameters = {
  config: {
    evm: Record<string, HyperlaneConfig>;
  };
  action: "deposit" | "withdraw";
  controllers: {
    inputs: Record<string, { value: string }>;
    reset: () => void;
    setValue: (name: string, value: string) => void;
  };
};

export function useBridgeState(params: UseBridgeStateParameters) {
  const { coins: allCoins, chain: dangoChain } = useConfig();

  const {
    action,
    controllers,
    config: { evm },
  } = params;

  const { current: networks } = useRef([
    { name: "Ethereum Network", id: "1", time: "16 blocks | 5-30 mins" },
    /*   { name: "Base Network", id: "8453", time: "5-30 mins" },
    { name: "Arbitrum Network", id: "42161", time: "5-30 mins" },
    { name: "Bitcoin Network", id: "bitcoin", time: "10-60 mins" },
    { name: "Solana Network", id: "solana", time: "2-10 mins" }, */
    ...(["Devnet", "Testnet"].includes(dangoChain.name)
      ? [{ name: "Sepolia Network", id: "11155111", time: "5-30 mins" }]
      : []),
  ]);

  const [network, setNetwork] = useState<string>();

  const [coin, setCoin] = useState<AnyCoin>();
  const changeCoin = useCallback((denom: string) => setCoin(allCoins.byDenom[denom]), [allCoins]);

  const connectors = useConnectors();
  const [connectorId, setConnectorId] = useStorage<string | null>("bridge_connector", {
    enabled: true,
    sync: true,
  });
  const connector = useMemo(
    () => connectors.find((c) => c.id === connectorId),
    [connectorId, connectors],
  );

  const coins = useMemo(() => {
    return Object.values(allCoins.byDenom).filter((c) =>
      ["USDC", "ETH", "USDT"].includes(c.symbol),
    );
  }, [allCoins]);

  const config = useMemo(() => {
    if (!network || !coin) return undefined;
    const chain = chains[network as keyof typeof chains];
    const bridger = (() => {
      if (network === "bitcoin") return undefined;
      if (network === "solana") return undefined;
      return evm[network as keyof typeof evm];
    })();

    const router = (() => {
      if (bridger && "hyperlane_domain" in bridger) {
        const router = bridger.warp_routes.find((r) =>
          r.symbol.toLowerCase().includes(coin.symbol.toLowerCase()),
        );
        if (!router) return undefined;

        return {
          remote: {
            warp: {
              domain: bridger.hyperlane_domain,
              contract: toAddr32(router.proxy_address),
            },
          },
          domain: bridger.hyperlane_domain,
          address: router.proxy_address,
          coin:
            typeof router.warp_route_type === "string"
              ? ("native" as const)
              : router.warp_route_type.erc20_collateral,
        };
      }
    })();

    return { chain, bridger, router };
  }, [network, coin]);

  const reset = useCallback(() => {
    setConnectorId(null);
    setCoin(undefined);
    setNetwork(undefined);
    controllers.reset();
  }, [controllers]);

  useEffect(() => {
    reset();
  }, [action]);

  return {
    action,
    config,
    coin,
    changeCoin,
    coins,
    network,
    setNetwork,
    networks,
    connector,
    setConnectorId,
    reset,
  };
}
