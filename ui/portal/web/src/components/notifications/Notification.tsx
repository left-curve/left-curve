import { useNavigate } from "@tanstack/react-router";
import { useApp } from "~/hooks/useApp";

import { formatNumber, formatUnits } from "@left-curve/dango/utils";
import {
  differenceInDays,
  differenceInHours,
  differenceInMinutes,
  format,
  isToday,
} from "date-fns";

import { m } from "~/paraglide/messages";

import { AddressVisualizer, IconInfo, twMerge } from "@left-curve/applets-kit";

import type { PropsWithChildren } from "react";
import type React from "react";
import type { Notifications } from "~/hooks/useNotifications";

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
  notification: Notifications;
};

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type NotificationTransferProps = {
  notification: Notifications<"transfer">;
};

const NotificationTransfer: React.FC<NotificationTransferProps> = ({ notification }) => {
  const navigate = useNavigate();
  const { settings, setNotificationMenuVisibility } = useApp();
  const { coin, type, fromAddress, toAddress, amount } = notification.data;
  const { formatNumberOptions } = settings;
  const isSent = type === "sent";

  const formattedAmount = formatNumber(formatUnits(amount, coin.decimals), formatNumberOptions);

  const originAddress = isSent ? toAddress : fromAddress;
  const targetAddress = isSent ? fromAddress : toAddress;

  const onNavigate = (url: string) => {
    setNotificationMenuVisibility(false);
    navigate({ to: url });
  };

  return (
    <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-rice-100 max-w-full">
      <div className="flex items-start gap-2 max-w-full overflow-hidden">
        <IconInfo className="text-gray-700 w-5 h-5 flex-shrink-0" />

        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <div className="flex gap-2">
            <span className="diatype-m-medium text-gray-700">
              {m["notifications.notification.transfer.title"]({ action: type })}
            </span>
            <span
              className={twMerge("diatype-m-bold flex-shrink-0", {
                "text-status-success": type === "received",
                "text-status-fail": type === "sent",
              })}
            >{`${isSent ? "−" : "+"}${formattedAmount}  ${coin.symbol}`}</span>
          </div>
          <div className="flex diatype-m-medium text-gray-500 flex-wrap items-center gap-1">
            <div className="flex flex-wrap items-center gap-1">
              <span>
                {m["notifications.notification.transfer.direction.first"]({ direction: type })}
              </span>
              <AddressVisualizer address={originAddress} withIcon onClick={onNavigate} />
            </div>
            <div className="flex flex-wrap items-center gap-1">
              <span>
                {m["notifications.notification.transfer.direction.second"]({ direction: type })}
              </span>
              <AddressVisualizer address={targetAddress} withIcon onClick={onNavigate} />{" "}
            </div>
          </div>
        </div>
      </div>
      <div className="diatype-sm-medium text-gray-500 min-w-fit">
        {formatNotificationTimestamp(new Date(notification.createdAt))}
      </div>
    </div>
  );
};

export const Notification = Object.assign(Container, {
  Transfer: NotificationTransfer,
});
