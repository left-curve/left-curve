import { Badge, Dot, Popover, twMerge } from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";

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
  const [globalStatus, setGlobalStatus] = useState<"error" | "success" | "warning">("success");
  const [websocketStatus, setWebsocketStatus] = useState<"error" | "success" | "warning">(
    "success",
  );
  const [chainStatus, setChainStatus] = useState<"error" | "success" | "warning">("warning");
  const [dexStatus, setDexStatus] = useState<"error" | "success" | "warning">("error");

  const blockNumber = 123456;

  return (
    <div className="fixed bottom-4 left-4 flex flex-col gap-2 z-50">
      <Popover
        showArrow={false}
        trigger={
          <Badge
            size="m"
            text={
              <div className="flex items-center gap-2">
                <Dot pulse />
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
              <div className="p-4 bg-surface-tertiary-rice min-w-[22rem] flex items-center justify-between rounded-md">
                <p className="text-ink-secondary-700 diatype-m-medium">
                  {m["statusBadge.websocket"]()}
                </p>
                <div
                  className={twMerge(
                    textColor[websocketStatus],
                    "diatype-xs-medium flex items-center gap-1",
                  )}
                >
                  {m["statusBadge.statusText"]({ status: websocketStatus })}
                  <Dot color={websocketStatus} />
                </div>
              </div>
              <div className="px-4 py-2 bg-surface-tertiary-rice min-w-[22rem] flex items-center justify-between rounded-md">
                <div className="flex flex-col">
                  <p className="text-ink-secondary-700 diatype-m-medium">
                    {m["statusBadge.chain"]()}
                  </p>
                  <p className="diatype-xs-medium text-ink-tertiary-500">#{blockNumber}</p>
                </div>
                <div
                  className={twMerge(
                    textColor[chainStatus],
                    "diatype-xs-medium flex items-center gap-1",
                  )}
                >
                  {m["statusBadge.statusText"]({ status: chainStatus })}
                  <Dot color={chainStatus} />
                </div>
              </div>
              <p className="p-4 bg-surface-tertiary-rice min-w-[22rem] flex items-center justify-between rounded-md">
                <p className="text-ink-secondary-700 diatype-m-medium">{m["statusBadge.dex"]()}</p>
                <div
                  className={twMerge(
                    textColor[dexStatus],
                    "diatype-xs-medium flex items-center gap-1",
                  )}
                >
                  {m["statusBadge.statusText"]({ status: dexStatus })}
                  <Dot color={dexStatus} />
                </div>
              </p>
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
