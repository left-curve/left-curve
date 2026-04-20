import { useEffect } from "react";
import type { TransportMode } from "@left-curve/dango/utils";

import {
  Badge,
  Button,
  CurrentBlock,
  Dot,
  IconLink,
  Popover,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useServiceStatus } from "@left-curve/store";

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
  const { navigate } = useApp();
  const {
    wsStatus,
    chainStatus,
    globalStatus,
    transportMode,
    isChainPaused,
    isReady,
  } = useServiceStatus({ upUrl: window.dango.urls.upUrl });

  useEffect(() => {
    if (isChainPaused === undefined) return;
    if (isChainPaused) navigate("/maintenance");
    if (!isChainPaused && window.location.pathname === "/maintenance") navigate("/");
  }, [isChainPaused]);

  if (!isReady) return null;

  return (
    <div className="hidden fixed bottom-4 left-4 lg:flex flex-col gap-2 z-50">
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
            <div className="flex items-center justify-between">
              <p className="h4-bold text-ink-primary-900">{m["statusBadge.status"]()}</p>
              <Button
                variant="link"
                className="py-0 h-fit flex gap-1 items-center"
                onClick={() =>
                  window.open("https://status.dango.exchange/", "_blank", "noopener,noreferrer")
                }
              >
                {m["statusBadge.details"]()}
                <IconLink className="w-4 h-4" />
              </Button>
            </div>
            <div className="flex flex-col gap-2">
              <WebSocketStatusSection wsStatus={wsStatus} transportMode={transportMode} />
              <ChainStatusSection chainStatus={chainStatus} />
            </div>
          </div>
        }
        classNames={{
          menu: "p-6 border border-outline-secondary-gray",
          panel: "px-4 py-2",
        }}
      />
    </div>
  );
};

type WebSocketStatusSectionProps = {
  wsStatus: "error" | "success" | "warning";
  transportMode: TransportMode;
};

const WebSocketStatusSection: React.FC<WebSocketStatusSectionProps> = ({
  wsStatus,
  transportMode,
}) => {
  const statusLabel =
    transportMode === "http-polling"
      ? m["statusBadge.httpPolling"]()
      : transportMode === "reconnecting" && wsStatus !== "success"
        ? m["statusBadge.reconnecting"]()
        : m["statusBadge.statusText"]({ status: wsStatus });

  return (
    <div className="p-4 bg-surface-tertiary-rice min-w-[22rem] flex items-center justify-between rounded-md">
      <p className="text-ink-secondary-700 diatype-m-medium">{m["statusBadge.websocket"]()}</p>
      <div className={twMerge(textColor[wsStatus], "diatype-xs-medium flex items-center gap-1")}>
        {statusLabel}
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
            skeleton: "h-[16.8px] w-12",
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
