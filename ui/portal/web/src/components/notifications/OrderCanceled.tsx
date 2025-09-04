import { useConfig } from "@left-curve/store";

import { Decimal } from "@left-curve/dango/utils";
import { Direction } from "@left-curve/dango/types";

import { OrderNotification } from "./OrderNotification";

import type { Notification } from "~/hooks/useNotifications";
import type React from "react";

type NotificationOrderCanceledProps = {
  notification: Notification<"orderCanceled">;
};

export const NotificationOrderCanceled: React.FC<NotificationOrderCanceledProps> = ({
  notification,
}) => {
  const { getCoinInfo } = useConfig();
  const { blockHeight, txHash } = notification;
  const { baseDenom, quoteDenom, price, kind, amount, direction } = notification.data;

  const opInfo =
    direction === Direction.Buy
      ? {
          amount: Decimal(amount).times(Decimal(price)).toFixed(),
          denom: quoteDenom,
        }
      : {
          amount: amount,
          denom: baseDenom,
        };

  const base = getCoinInfo(baseDenom);
  const quote = getCoinInfo(quoteDenom);
  const deposit = getCoinInfo(opInfo.denom);

  return (
    <OrderNotification
      details={{ price, amount: opInfo.amount, coin: deposit }}
      base={base}
      quote={quote}
      kind={kind}
      blockHeight={blockHeight}
      txHash={txHash}
    />
  );
};
