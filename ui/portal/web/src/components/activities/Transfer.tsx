import { useConfig } from "@left-curve/store";
import { useRouter } from "@tanstack/react-router";
import { forwardRef, useImperativeHandle } from "react";

import { formatNumber, formatUnits } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  AddressVisualizer,
  IconReceived,
  IconSent,
  Modals,
  PairAssets,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";

type ActivityTransferProps = {
  activity: ActivityRecord<"transfer">;
};

export const ActivityTransfer = forwardRef<ActivityRef, ActivityTransferProps>(
  ({ activity }, ref) => {
    const { settings, showModal } = useApp();
    const { navigate } = useRouter();
    const { getCoinInfo } = useConfig();
    const { blockHeight, txHash, createdAt } = activity;
    const { coins, type, fromAddress, toAddress } = activity.data;
    const { formatNumberOptions } = settings;
    const isSent = type === "sent";
    const Icon = isSent ? IconSent : IconReceived;

    useImperativeHandle(ref, () => ({
      onClick: () => {
        showModal(Modals.ActivityTransfer, {
          navigate,
          blockHeight,
          txHash,
          coins,
          action: type,
          from: fromAddress,
          to: toAddress,
          time: createdAt,
        });
      },
    }));

    const onNavigate = (url: string) => {
      navigate({ to: url });
    };

    return (
      <div className="flex items-start gap-2 max-w-full overflow-hidden cursor-pointer">
        <div className="flex items-center justify-center bg-surface-quaternary-rice min-w-7 min-h-7 w-7 h-7 rounded-sm">
          <Icon
            className={twMerge(isSent ? "text-primitives-red-light-600" : "text-brand-green")}
          />
        </div>

        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <span className="diatype-m-medium text-ink-secondary-700">
            {m["activities.activity.transfer.title"]({ action: type })}
          </span>
          <div className="flex flex-col items-start">
            {Object.entries(coins).map(([denom, amount]) => {
              const coin = getCoinInfo(denom);
              return (
                <p
                  key={denom}
                  className={twMerge(
                    "diatype-m-bold flex-shrink-0 flex items-center justify-center gap-1",
                    {
                      "text-status-success": type === "received",
                      "text-status-fail": type === "sent",
                    },
                  )}
                >
                  <span>
                    {coin.type === "lp" ? (
                      <PairAssets assets={[coin.base, coin.quote]} />
                    ) : (
                      <img
                        src={coin.logoURI}
                        alt={coin.symbol}
                        className="w-5 h-5 min-w-5 min-h-5 select-none drag-none"
                        loading="lazy"
                      />
                    )}
                  </span>
                  {`${isSent ? "−" : "+"}${formatNumber(formatUnits(amount, coin.decimals), formatNumberOptions)}  ${coin.symbol}`}
                </p>
              );
            })}
          </div>
          <div className="flex flex-col diatype-m-medium text-ink-tertiary-500 items-start gap-1">
            <div className="flex flex-wrap items-center gap-1">
              <span>{m["activities.activity.transfer.direction.first"]()}</span>
              <AddressVisualizer
                classNames={{ container: "address-visualizer" }}
                address={fromAddress}
                withIcon
                onClick={onNavigate}
              />
            </div>
            <div className="flex flex-wrap items-center gap-1">
              <span>{m["activities.activity.transfer.direction.second"]()}</span>
              <AddressVisualizer
                classNames={{ container: "address-visualizer" }}
                address={toAddress}
                withIcon
                onClick={onNavigate}
              />{" "}
            </div>
          </div>
        </div>
      </div>
    );
  },
);
