import { useNavigate } from "@tanstack/react-router";
import { useConfig } from "@left-curve/store";

import { formatNumber, formatUnits } from "@left-curve/dango/utils";
import {
  differenceInDays,
  differenceInHours,
  differenceInMinutes,
  format,
  isToday,
} from "date-fns";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  AddressVisualizer,
  IconClose,
  IconInfo,
  PairAssets,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";

import type { PropsWithChildren } from "react";
import type React from "react";
import { type Notification as NotificationType, useNotifications } from "~/hooks/useNotifications";

const formatNotificationTimestamp = (timestamp: Date): string => {
  const now = new Date();
  if (isToday(timestamp)) {
    const minutesDifference = differenceInMinutes(now, timestamp);
    if (minutesDifference < 1) {
      return "1m";
    }

    if (minutesDifference < 60) {
      return `${minutesDifference}m`;
    }

    const hoursDifference = differenceInHours(now, timestamp);
    if (hoursDifference < 24) {
      return `${hoursDifference}h`;
    }
  }

  const daysDifference = differenceInDays(now, timestamp);
  if (daysDifference === 1) {
    return "1d";
  }

  return format(timestamp, "MM/dd");
};

export type NotificationProps = {
  notification: Notification[keyof Notification];
};

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type NotificationTransferProps = {
  notification: NotificationType<"transfer">;
};

const NotificationTransfer: React.FC<NotificationTransferProps> = ({ notification }) => {
  const navigate = useNavigate();
  const { settings, setNotificationMenuVisibility } = useApp();
  const { deleteNotification } = useNotifications();
  const { getCoinInfo } = useConfig();
  const { id, blockHeight, createdAt, txHash } = notification;
  const { coins, type, fromAddress, toAddress } = notification.data;
  const { formatNumberOptions } = settings;
  const isSent = type === "sent";

  const originAddress = isSent ? fromAddress : toAddress;
  const targetAddress = isSent ? toAddress : fromAddress;

  const onNavigate = (url: string) => {
    setNotificationMenuVisibility(false);
    navigate({ to: url });
  };

  return (
    <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-surface-quaternary-rice max-w-full group">
      <div
        className="flex items-start gap-2 max-w-full overflow-hidden cursor-pointer"
        onClick={(event) => {
          const element = event.target as HTMLElement;
          if (element.closest(".address-visualizer") || element.closest(".remove-notification")) {
            return;
          }
          onNavigate(txHash ? `/tx/${txHash}` : `/block/${blockHeight}`);
        }}
      >
        <IconInfo className="text-secondary-700 w-5 h-5 flex-shrink-0" />

        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <span className="diatype-m-medium text-secondary-700">
            {m["notifications.notification.transfer.title"]({ action: type })}
          </span>
          <div className="flex gap-2">
            {Object.entries(coins).map(([denom, amount]) => {
              const coin = getCoinInfo(denom);
              return (
                <p
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
                        className="w-5 h-5 select-none drag-none"
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
          <div className="flex diatype-m-medium text-tertiary-500 flex-wrap items-center gap-1">
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
      <div className="flex flex-col diatype-sm-medium text-tertiary-500 min-w-fit items-center relative">
        <IconClose
          className="absolute w-6 h-6 cursor-pointer group-hover:block hidden top-[-26px] remove-notification"
          onClick={() => deleteNotification(id)}
        />
        <p>{formatNotificationTimestamp(new Date(createdAt))}</p>
      </div>
    </div>
  );
};

type NotificationAccountProps = {
  notification: NotificationType<"account">;
};

const NotificationAccount: React.FC<NotificationAccountProps> = ({ notification }) => {
  const navigate = useNavigate();
  const { setNotificationMenuVisibility } = useApp();
  const { deleteNotification } = useNotifications();
  const { address, accountType } = notification.data;

  const onNavigate = (url: string) => {
    setNotificationMenuVisibility(false);
    navigate({ to: url });
  };

  return (
    <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-surface-quaternary-rice max-w-full group">
      <div className="flex items-start gap-2 max-w-full overflow-hidden">
        <IconInfo className="text-secondary-700 w-5 h-5 flex-shrink-0" />
        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <span className="diatype-m-medium text-secondary-700 capitalize">
            {m["notifications.notification.account.title"]({ accountType })}
          </span>
          <AddressVisualizer address={address} withIcon onClick={onNavigate} />
        </div>
      </div>
      <div className="flex flex-col diatype-sm-medium text-tertiary-500 min-w-fit items-center relative">
        <IconClose
          className="absolute w-6 h-6 cursor-pointer group-hover:block hidden top-[-26px]"
          onClick={() => deleteNotification(notification.id)}
        />
        <p>{formatNotificationTimestamp(new Date(notification.createdAt))}</p>
      </div>
    </div>
  );
};

export const Notification = Object.assign(Container, {
  Transfer: NotificationTransfer,
  Account: NotificationAccount,
});
