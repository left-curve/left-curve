import { useCallback, useEffect, useMemo, useState } from "react";
import { useConnectors } from "./useConnectors.js";
import { useConfig } from "./useConfig.js";
import { useStorage } from "./useStorage.js";

import { chains } from "../hyperlane.js";
import { toAddr32 } from "@left-curve/sdk/hyperlane";

import type { AnyCoin } from "../types/coin.js";
import type { HyperlaneConfig, HyperlaneEvmChainConfig } from "@left-curve/types";
import { useAccount } from "./useAccount.js";

const SUPPORTED_BRIDGE_SYMBOLS = new Set(["USDC"]);

function normalizeSymbol(symbol: string) {
  return symbol.toUpperCase();
}

function hasSupportedRoute(bridger: HyperlaneEvmChainConfig | undefined, coin?: AnyCoin) {
  if (!bridger) return false;

  return bridger.routes.some((route) => {
    const routeSymbol = normalizeSymbol(route.symbol);
    if (!SUPPORTED_BRIDGE_SYMBOLS.has(routeSymbol)) return false;
    if (!coin) return true;
    return routeSymbol === normalizeSymbol(coin.symbol);
  });
}

export type UseBridgeStateParameters = {
  config: HyperlaneConfig;
  action: "deposit" | "withdraw";
  controllers: {
    inputs: Record<string, { value: string }>;
    reset: () => void;
    setValue: (name: string, value: string) => void;
  };
};

export function useBridgeState(params: UseBridgeStateParameters) {
  const { coins: allCoins } = useConfig();
  const { isConnected } = useAccount();

  const {
    action,
    controllers,
    config: { evm },
  } = params;

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

  const configuredNetworks = useMemo(() => {
    return Object.entries(evm)
      .filter(([, bridger]) => hasSupportedRoute(bridger))
      .sort(([, left], [, right]) => left.order - right.order)
      .map(([id, bridger]) => ({
        id,
        name: bridger.name,
        time: bridger.estimatedTime,
      }));
  }, [evm]);

  const networks = useMemo(() => {
    return configuredNetworks.filter(({ id }) => hasSupportedRoute(evm[id], coin));
  }, [coin, configuredNetworks, evm]);

  const coins = useMemo(() => {
    const selectedNetworks = network
      ? configuredNetworks.filter(({ id }) => id === network)
      : configuredNetworks;

    const supportedSymbols = selectedNetworks.reduce((symbols, { id }) => {
      const bridger = evm[id];
      if (!bridger) return symbols;

      for (const route of bridger.routes) {
        const routeSymbol = normalizeSymbol(route.symbol);
        if (SUPPORTED_BRIDGE_SYMBOLS.has(routeSymbol)) symbols.add(routeSymbol);
      }

      return symbols;
    }, new Set<string>());

    return Object.values(allCoins.byDenom).filter((c) =>
      supportedSymbols.has(normalizeSymbol(c.symbol)),
    );
  }, [allCoins, configuredNetworks, evm, network]);

  const config = useMemo(() => {
    if (!network || !coin) return undefined;
    const chain = chains[network as keyof typeof chains];
    const bridger = (() => {
      if (network === "bitcoin") return undefined;
      if (network === "solana") return undefined;
      return evm[network as keyof typeof evm];
    })();

    const router = (() => {
      if (!SUPPORTED_BRIDGE_SYMBOLS.has(normalizeSymbol(coin.symbol))) return undefined;

      if (bridger) {
        const router = bridger.routes.find(
          (r) => normalizeSymbol(r.symbol) === normalizeSymbol(coin.symbol),
        );
        if (!router) return undefined;

        return {
          remote: {
            warp: {
              domain: bridger.domain,
              contract: toAddr32(router.routerAddress),
            },
          },
          domain: bridger.domain,
          address: router.routerAddress,
          coin: router.tokenAddress,
        };
      }
    })();

    return { chain, bridger, router };
  }, [coin, evm, network]);

  const reset = useCallback(() => {
    setConnectorId(null);
    setCoin(undefined);
    setNetwork(undefined);
    controllers.reset();
  }, [controllers]);

  useEffect(() => {
    reset();
  }, [action, isConnected]);

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
