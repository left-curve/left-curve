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
const DEPOSIT_COIN_SYMBOL = "USDC";
const ETHEREUM_MAINNET_CHAIN_ID = 1;
const ARBITRUM_CHAIN_IDS = new Set(["42161", "421614"]);

type BridgeAction = "deposit" | "withdraw";

function normalizeSymbol(symbol: string) {
  return symbol.toUpperCase();
}

function isSupportedRoute(
  action: BridgeAction,
  bridger: HyperlaneEvmChainConfig,
  route: HyperlaneEvmChainConfig["routes"][number],
) {
  const routeSymbol = normalizeSymbol(route.symbol);
  if (SUPPORTED_BRIDGE_SYMBOLS.has(routeSymbol)) return true;

  return (
    action === "withdraw" &&
    bridger.chainId === ETHEREUM_MAINNET_CHAIN_ID &&
    route.type === "native" &&
    routeSymbol === "ETH"
  );
}

function hasSupportedRoute(
  action: BridgeAction,
  bridger: HyperlaneEvmChainConfig | undefined,
  coin?: AnyCoin,
) {
  if (!bridger) return false;

  return bridger.routes.some((route) => {
    const routeSymbol = normalizeSymbol(route.symbol);
    if (!isSupportedRoute(action, bridger, route)) return false;
    if (!coin) return true;
    return routeSymbol === normalizeSymbol(coin.symbol);
  });
}

function isDepositCoin(coin: AnyCoin | undefined) {
  return coin ? normalizeSymbol(coin.symbol) === DEPOSIT_COIN_SYMBOL : false;
}

export type UseBridgeStateParameters = {
  config: HyperlaneConfig;
  action: BridgeAction;
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
      .filter(([, bridger]) => hasSupportedRoute(action, bridger))
      .sort(([, left], [, right]) => left.order - right.order)
      .map(([id, bridger]) => ({
        id,
        name: bridger.name,
        time: bridger.estimatedTime,
      }));
  }, [action, evm]);

  const defaultDepositCoin = useMemo(() => {
    return Object.values(allCoins.byDenom).find(isDepositCoin);
  }, [allCoins.byDenom]);

  const getDefaultDepositNetwork = useCallback(
    (nextCoin: AnyCoin | undefined) => {
      const supportedNetworks = configuredNetworks.filter(({ id }) =>
        hasSupportedRoute("deposit", evm[id], nextCoin),
      );

      return (
        supportedNetworks.find(({ id }) => ARBITRUM_CHAIN_IDS.has(id)) ?? supportedNetworks[0]
      )?.id;
    },
    [configuredNetworks, evm],
  );

  const changeCoin = useCallback(
    (denom: string) => {
      const nextCoin = allCoins.byDenom[denom];
      if (action === "deposit" && !isDepositCoin(nextCoin)) return;

      setCoin(nextCoin);

      if (action !== "deposit") return;
      setNetwork(getDefaultDepositNetwork(nextCoin));
    },
    [action, allCoins, getDefaultDepositNetwork],
  );

  const networks = useMemo(() => {
    return configuredNetworks.filter(({ id }) => hasSupportedRoute(action, evm[id], coin));
  }, [action, coin, configuredNetworks, evm]);

  const coins = useMemo(() => {
    const selectedNetworks = network
      ? configuredNetworks.filter(({ id }) => id === network)
      : configuredNetworks;

    const supportedSymbols = selectedNetworks.reduce((symbols, { id }) => {
      const bridger = evm[id];
      if (!bridger) return symbols;

      for (const route of bridger.routes) {
        const routeSymbol = normalizeSymbol(route.symbol);
        if (isSupportedRoute(action, bridger, route)) symbols.add(routeSymbol);
      }

      return symbols;
    }, new Set<string>());

    return Object.values(allCoins.byDenom).filter((c) =>
      supportedSymbols.has(normalizeSymbol(c.symbol)),
    );
  }, [action, allCoins, configuredNetworks, evm, network]);

  const config = useMemo(() => {
    if (!network || !coin) return undefined;
    const chain = chains[network as keyof typeof chains];
    const bridger = (() => {
      if (network === "bitcoin") return undefined;
      if (network === "solana") return undefined;
      return evm[network as keyof typeof evm];
    })();

    const router = (() => {
      if (bridger) {
        const router = bridger.routes.find(
          (route) =>
            isSupportedRoute(action, bridger, route) &&
            normalizeSymbol(route.symbol) === normalizeSymbol(coin.symbol),
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
  }, [action, coin, evm, network]);

  const reset = useCallback(() => {
    setConnectorId(null);
    setCoin(undefined);
    setNetwork(undefined);
    controllers.reset();
  }, [controllers]);

  useEffect(() => {
    reset();
  }, [action, isConnected]);

  useEffect(() => {
    if (action !== "deposit" || !defaultDepositCoin) return;

    setCoin((currentCoin) => (isDepositCoin(currentCoin) ? currentCoin : defaultDepositCoin));
    setNetwork((currentNetwork) => currentNetwork ?? getDefaultDepositNetwork(defaultDepositCoin));
  }, [action, defaultDepositCoin, getDefaultDepositNetwork]);

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
