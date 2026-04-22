import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { usePublicClient } from "./usePublicClient.js";
import { useQuery } from "@tanstack/react-query";

import type { TransportMode } from "@left-curve/dango/utils";

type ServiceStatus = "success" | "error" | "warning";

type UseServiceStatusParameters = {
  upUrl?: string;
};

export function useServiceStatus(parameters?: UseServiceStatusParameters) {
  const { upUrl } = parameters ?? {};
  const [enableWsCheck, setEnableWsCheck] = useState(false);
  const publicClient = usePublicClient();

  useEffect(() => {
    const t = setTimeout(() => setEnableWsCheck(true), 1_000);
    return () => clearTimeout(t);
  }, []);

  const { data: wsIsConnected, isFetched: isWsChecked } = useQuery({
    enabled: enableWsCheck,
    queryKey: ["websocket_status"],
    queryFn: async () => publicClient.subscribe?.getClientStatus?.().isConnected,
    refetchInterval: 5_000,
  });

  const { data: isChainPaused, isFetched: isChainChecked } = useQuery({
    enabled: !!upUrl,
    queryKey: ["chain_status"],
    queryFn: async () => {
      try {
        const response = await fetch(upUrl!);
        if (!response.ok) throw new Error("request failed");
        const { is_running } = await response.json();
        return !is_running;
      } catch (_) {
        return true;
      }
    },
    refetchInterval: 30_000,
  });

  const { data: isDexPaused, isFetched: isDexChecked } = useQuery({
    queryKey: ["dex_status"],
    queryFn: async () => await publicClient.dexStatus(),
    refetchInterval: 30_000,
  });

  const transportMode = useSyncExternalStore<TransportMode>(
    (callback) => {
      const emitter = publicClient.subscribe?.emitter;
      if (!emitter) return () => {};
      emitter.on("transport-mode", callback);
      return () => emitter.off("transport-mode", callback);
    },
    () => {
      const isConnected = publicClient.subscribe?.getClientStatus?.().isConnected;
      if (isConnected) return "ws";
      return "reconnecting";
    },
    () => "ws",
  );

  const wsStatus: ServiceStatus = wsIsConnected
    ? "success"
    : transportMode === "http-polling"
      ? "warning"
      : "error";
  const chainStatus: ServiceStatus = isChainPaused ? "error" : "success";
  const dexStatus: ServiceStatus = isChainPaused || isDexPaused ? "error" : "success";

  const globalStatus = useMemo<ServiceStatus>(() => {
    if (chainStatus === "error") return "error";
    if (wsStatus === "error") return "warning";
    return "success";
  }, [chainStatus, wsStatus]);

  return {
    wsStatus,
    chainStatus,
    dexStatus,
    globalStatus,
    transportMode,
    isChainPaused,
    isReady: isWsChecked && isChainChecked && isDexChecked,
  };
}
