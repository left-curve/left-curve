import { useConfig } from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { OrderNotification } from "./OrderNotification";

import type { Notification } from "~/hooks/useNotifications";
import type React from "react";

type NotificationOrderCreatedProps = {
  notification: Notification<"orderCreated">;
};

export const NotificationOrderCreated: React.FC<NotificationOrderCreatedProps> = ({
  notification,
}) => {
  const { getCoinInfo } = useConfig();
  const { blockHeight, txHash } = notification;
  const { quote_denom, base_denom, price, kind, deposit: depositInfo } = notification.data;

  const base = getCoinInfo(base_denom);
  const quote = getCoinInfo(quote_denom);
  const deposit = getCoinInfo(depositInfo.denom);

  return (
    <OrderNotification
      title={m["notifications.notification.orderCreated.title"]({ orderType: kind })}
      details={{ price, amount: depositInfo.amount, coin: deposit }}
      base={base}
      quote={quote}
      kind={kind}
      blockHeight={blockHeight}
      txHash={txHash}
    />
  );
};
