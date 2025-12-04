import { useCallback, useEffect, useMemo, useState } from "react";

import type { AnyCoin } from "../types/coin.js";
import { useConnectors } from "./useConnectors.js";
import { useSubmitTx } from "./useSubmitTx.js";
import { useQuery } from "@tanstack/react-query";
import { useConfig } from "./useConfig.js";

import type { EIP1193Provider } from "../types/eip1193.js";

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
  const { coins: allCoins } = useConfig();
  const [coin, setCoin] = useState<AnyCoin | null>(null);
  const [network, setNetwork] = useState<string | null>(null);
  const [connectorId, setConnectorId] = useState<string | null>(null);
  const [getAmount, setGetAmount] = useState<string>("0");
  const connectors = useConnectors();
  const { inputs } = controllers;

  const operationAmount = inputs.amount?.value || "0";

  const changeCoin = useCallback((denom: string) => setCoin(allCoins.byDenom[denom]), [allCoins]);

  const connector = useMemo(
    () => connectors.find((c) => c.id === connectorId),
    [connectorId, connectors],
  );

  const coins = useMemo(() => {
    return Object.values(allCoins.byDenom).filter((c) =>
      ["USDC", "ETH", "USDT"].includes(c.symbol),
    );
  }, [allCoins]);

  const deposit = useSubmitTx({
    mutation: {
      mutationFn: async () => {},
    },
  });

  const withdraw = useSubmitTx({
    mutation: {
      mutationFn: async () => {},
    },
  });

  const { data: depositAddress } = useQuery({
    queryKey: ["bridge", "depositAddress", network],
    queryFn: async () => {
      if (!["bitcoin"].includes(network as string)) return null;
      return "address";
    },
  });

  const walletAddress = useQuery({
    enabled: action === "deposit" && !!connector,
    queryKey: ["bridge", "connectedAddress", connectorId],
    queryFn: async () => {
      if (!connector) return null;
      const provider = await (
        connector as unknown as { getProvider: () => Promise<EIP1193Provider> }
      ).getProvider();
      const [account] = await provider.request({ method: "eth_requestAccounts" });
      return account;
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
  };
}
