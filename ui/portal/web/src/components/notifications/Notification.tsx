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
    <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-rice-100 max-w-full">
      <div className="flex items-start gap-2 max-w-full overflow-hidden">
        <IconInfo className="text-gray-700 w-5 h-5 flex-shrink-0" />

        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <p className="diatype-m-medium text-gray-700">
            {m["notifications.notification.transfer.title"]({ action: type })}
          </p>
          <div className="flex diatype-m-medium text-gray-500 gap-1 flex-wrap">
            <span
              className={twMerge("diatype-m-bold flex-shrink-0", {
                "text-status-success": type === "received",
                "text-status-fail": type === "sent",
              })}
            >{`${isSent ? "−" : "+"} ${formatUnits(amount, coin.decimals)} ${coin.symbol}`}</span>
            <span>{m["notifications.notification.transfer.direction"]({ direction: type })}</span>
            <AddressVisualizer address={address} withIcon />
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
