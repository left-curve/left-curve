import { useConfig } from "@left-curve/store";
import { Direction } from "@left-curve/dango/types";

import { OrderNotification } from "./OrderNotification";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { Notification } from "~/hooks/useNotifications";
import type React from "react";

type NotificationOrderFilledProps = {
  notification: Notification<"orderFilled">;
};

export const NotificationOrderFilled: React.FC<NotificationOrderFilledProps> = ({
  notification,
}) => {
  const { getCoinInfo } = useConfig();
  const { blockHeight, txHash } = notification;
  const {
    kind,
    base_denom,
    quote_denom,
    clearing_price,
    cleared,
    direction,
    filled_base,
    filled_quote,
  } = notification.data;

  const opInfo =
    direction === Direction.Buy
      ? {
          amount: filled_quote,
          denom: quote_denom,
        }
      : {
          amount: filled_base,
          denom: base_denom,
        };

  const base = getCoinInfo(base_denom);
  const quote = getCoinInfo(quote_denom);
  const deposit = getCoinInfo(opInfo.denom);

  return (
    <OrderNotification
      title={m["notifications.notification.orderFilled.title"]({ orderType: kind })}
      details={{ price: clearing_price, amount: opInfo.amount, coin: deposit }}
      base={base}
      quote={quote}
      kind={kind}
      blockHeight={blockHeight}
      txHash={txHash}
    />
  );
};
