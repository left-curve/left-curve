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

import type { Notification } from "~/hooks/useNotifications";
import type { NotificationRef } from "./Notification";

type NotificationTransferProps = {
  notification: Notification<"transfer">;
};

export const NotificationTransfer = forwardRef<NotificationRef, NotificationTransferProps>(
  ({ notification }, ref) => {
    const { settings, showModal, setNotificationMenuVisibility } = useApp();
    const { navigate } = useRouter();
    const { getCoinInfo } = useConfig();
    const { blockHeight, txHash, createdAt } = notification;
    const { coins, type, fromAddress, toAddress } = notification.data;
    const { formatNumberOptions } = settings;
    const isSent = type === "sent";

    const originAddress = isSent ? fromAddress : toAddress;
    const targetAddress = isSent ? toAddress : fromAddress;

    const Icon = isSent ? IconSent : IconReceived;

    useImperativeHandle(ref, () => ({
      onClick: () => {
        showModal(Modals.NotificationSentAndReceived, {
          blockHeight,
          txHash,
          coins,
          action: type,
          from: originAddress,
          to: targetAddress,
          time: createdAt,
        });
      },
    }));

    const onNavigate = (url: string) => {
      setNotificationMenuVisibility(false);
      navigate({ to: url });
    };

    return (
      <div className="flex items-start gap-2 max-w-full overflow-hidden cursor-pointer">
        <div className="flex items-center justify-center bg-quaternary-rice min-w-7 min-h-7 w-7 h-7 rounded-sm">
          <Icon className={twMerge(isSent ? "text-red-bean-600" : "text-brand-green")} />
        </div>

        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <span className="diatype-m-medium text-secondary-700">
            {m["notifications.notification.transfer.title"]({ action: type })}
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
                  {`${isSent ? "−" : "+"}${formatNumber(formatUnits(amount, coin.decimals), {
                    ...formatNumberOptions,
                    maxSignificantDigits: 4,
                  })}  ${coin.symbol}`}
                </p>
              );
            })}
          </div>
          <div className="flex flex-col diatype-m-medium text-tertiary-500 items-start gap-1">
            <div className="flex flex-wrap items-center gap-1">
              <span>
                {m["notifications.notification.transfer.direction.first"]({ direction: type })}
              </span>
              <AddressVisualizer
                classNames={{ container: "address-visualizer" }}
                address={originAddress}
                withIcon
                onClick={onNavigate}
              />
            </div>
            <div className="flex flex-wrap items-center gap-1">
              <span>
                {m["notifications.notification.transfer.direction.second"]({ direction: type })}
              </span>
              <AddressVisualizer
                classNames={{ container: "address-visualizer" }}
                address={targetAddress}
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
