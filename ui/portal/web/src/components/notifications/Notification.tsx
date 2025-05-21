import { AddressVisualizer, IconInfo, twMerge } from "@left-curve/applets-kit";
import {
  differenceInDays,
  differenceInHours,
  differenceInMinutes,
  format,
  isToday,
} from "date-fns";

import { formatUnits } from "@left-curve/dango/utils";
import { m } from "~/paraglide/messages";

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
  const { coin, type, fromAddress, toAddress, amount } = notification.data;
  const isSent = type === "sent";

  const address = isSent ? toAddress : fromAddress;
  return (
    <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-rice-100">
      <div className="flex items-start gap-2">
        <IconInfo className="text-gray-700 w-5 h-5" />

        <div className="flex flex-col">
          <p className="diatype-m-medium text-gray-700">
            {m["notifications.notification.transfer.title"]({ action: type })}
          </p>
          <div className="flex flex-wrap diatype-m-medium text-gray-500 gap-1">
            <span
              className={twMerge("diatype-m-bold", {
                "text-status-success": type === "received",
                "text-status-fail": type === "sent",
              })}
            >{`${isSent ? "−" : "+"} ${formatUnits(amount, coin.decimals)} ${coin.symbol}`}</span>
            <span>{m["notifications.notification.transfer.direction"]({ direction: type })}</span>
            <AddressVisualizer address={address} withIcon />
          </div>
        </div>
      </div>
      <div className="diatype-sm-medium text-gray-500">
        {formatNotificationTimestamp(new Date(notification.createdAt))}
      </div>
    </div>
  );
};

export const Notification = Object.assign(Container, {
  Transfer: NotificationTransfer,
});
