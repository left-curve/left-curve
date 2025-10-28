import { useEffect, useMemo, useState } from "react";
import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import { Badge, CurrentBlock, Dot, Popover, twMerge, useApp } from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";

const badgeColor = {
  error: "light-red",
  success: "light-green",
  warning: "warning",
};

const textColor = {
  error: "text-utility-error-500",
  success: "text-utility-success-500",
  warning: "text-utility-warning-500",
};

export const StatusBadge: React.FC = () => {
  const [enableWsCheck, setEnableWsCheck] = useState(() => {
    setTimeout(() => setEnableWsCheck(true), 1_000);
    return false;
  });
  const publicClient = usePublicClient();
  const { navigate } = useApp();

  const { data: wsIsConnected, isFetched: isWsChecked } = useQuery({
    enabled: enableWsCheck,
    queryKey: ["websocket_status"],
    queryFn: async () => publicClient.subscribe?.getClientStatus?.().isConnected,
    refetchInterval: 30_000,
  });

  const { data: isChainPaused, isFetched: isChainChecked } = useQuery({
    queryKey: ["chain_status"],
    queryFn: async () => {
      try {
        const response = await fetch(window.dango.urls.upUrl);
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

  const wsStatus = wsIsConnected ? "success" : "error";
  const chainStatus = isChainPaused ? "error" : "success";
  const dexStatus = isDexPaused ? "error" : "success";

  const globalStatus = useMemo(() => {
    if (chainStatus === "error") return "error";
    if (dexStatus === "error" || wsStatus === "error") return "warning";
    return "success";
  }, [dexStatus, chainStatus, wsStatus]);

  useEffect(() => {
    if (isChainPaused === undefined) return;
    if (isChainPaused) navigate("/maintenance");
    if (!isChainPaused && window.location.pathname === "/maintenance") navigate("/");
  }, [isChainPaused]);

  if (!isWsChecked || !isChainChecked || !isDexChecked) return null;

  return (
    <div className="fixed bottom-4 left-4 flex flex-col gap-2 z-50">
      <Popover
        showArrow={false}
        trigger={
          <Badge
            size="m"
            text={
              <div className="flex items-center gap-2">
                <Dot pulse color={globalStatus} />
                {m["statusBadge.statusText"]({ status: globalStatus })}
              </div>
            }
            color={badgeColor[globalStatus] as "light-red" | "light-green" | "warning"}
          />
        }
        menu={
          <div className="flex flex-col gap-4">
            <p className="h4-bold text-ink-primary-900">{m["statusBadge.status"]()}</p>
            <div className="flex flex-col gap-2">
              <WebSocketStatusSection wsStatus={wsStatus} />
              <ChainStatusSection chainStatus={chainStatus} />
              <DexStatusSection dexStatus={dexStatus} />
            </div>
          </div>
        }
        classNames={{
          menu: "p-6 shadow-none border border-outline-secondary-gray",
          panel: "px-4 py-2",
        }}
      />
    </div>
  );
};

type WebSocketStatusSectionProps = {
  wsStatus: "error" | "success" | "warning";
};

const WebSocketStatusSection: React.FC<WebSocketStatusSectionProps> = ({ wsStatus }) => {
  return (
    <div className="p-4 bg-surface-tertiary-rice min-w-[22rem] flex items-center justify-between rounded-md">
      <p className="text-ink-secondary-700 diatype-m-medium">{m["statusBadge.websocket"]()}</p>
      <div className={twMerge(textColor[wsStatus], "diatype-xs-medium flex items-center gap-1")}>
        {m["statusBadge.statusText"]({ status: wsStatus })}
        <Dot color={wsStatus} />
      </div>
    </div>
  );
};

type ChainStatusSectionProps = {
  chainStatus: "error" | "success" | "warning";
};

const ChainStatusSection: React.FC<ChainStatusSectionProps> = ({ chainStatus }) => {
  return (
    <div className="px-4 py-2 bg-surface-tertiary-rice min-w-[22rem] flex items-center justify-between rounded-md">
      <div className="flex flex-col">
        <p className="text-ink-secondary-700 diatype-m-medium">{m["statusBadge.chain"]()}</p>
        <CurrentBlock
          classNames={{
            container: "diatype-xs-medium text-ink-tertiary-500",
            skeleton: "h-4 w-12",
          }}
        />
      </div>
      <div className={twMerge(textColor[chainStatus], "diatype-xs-medium flex items-center gap-1")}>
        {m["statusBadge.statusText"]({ status: chainStatus })}
        <Dot color={chainStatus} />
      </div>
    </div>
  );
};

type DexStatusSectionProps = {
  dexStatus: "error" | "success" | "warning";
};

const DexStatusSection: React.FC<DexStatusSectionProps> = ({ dexStatus }) => {
  return (
    <div className="p-4 bg-surface-tertiary-rice min-w-[22rem] flex items-center justify-between rounded-md">
      <p className="text-ink-secondary-700 diatype-m-medium">{m["statusBadge.dex"]()}</p>
      <div className={twMerge(textColor[dexStatus], "diatype-xs-medium flex items-center gap-1")}>
        {m["statusBadge.statusText"]({ status: dexStatus })}
        <Dot color={dexStatus} />
      </div>
    </div>
  );
};
