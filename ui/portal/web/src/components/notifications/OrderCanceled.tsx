import { useConfig } from "@left-curve/store";

import { calculatePrice, Decimal, formatNumber, formatUnits } from "@left-curve/dango/utils";
import { Direction, OrderType } from "@left-curve/dango/types";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { OrderNotification } from "./OrderNotification";

import type { Notification } from "~/hooks/useNotifications";
import type React from "react";
import { PairAssets, twMerge, useApp } from "@left-curve/applets-kit";

type NotificationOrderCanceledProps = {
  notification: Notification<"orderCanceled">;
};

export const NotificationOrderCanceled: React.FC<NotificationOrderCanceledProps> = ({
  notification,
}) => {
  const { getCoinInfo } = useConfig();
  const { createdAt, blockHeight } = notification;
  const { id, quote_denom, base_denom, price, kind, direction, amount, refund, remaining } =
    notification.data;
  const { settings, showModal } = useApp();
  const { formatNumberOptions } = settings;

  const isLimit = kind === OrderType.Limit;

  const base = getCoinInfo(base_denom);
  const quote = getCoinInfo(quote_denom);
  const refundCoin = getCoinInfo(refund.denom);

  const limitPrice = isLimit
    ? calculatePrice(price, { base: base.decimals, quote: quote.decimals }, formatNumberOptions)
    : null;

  const filled = Decimal(amount).minus(remaining).toFixed();

  return (
    <OrderNotification
      kind={kind}
      onClick={() =>
        showModal("notification-spot-action-order", {
          base,
          quote,
          blockHeight,
          action: direction === "ask" ? "sell" : "buy",
          status: "canceled",
          order: {
            id,
            limitPrice,
            type: kind,
            timeCanceled: createdAt,
            filledAmount: formatNumber(formatUnits(filled, base.decimals), formatNumberOptions),
            refund: [{ ...refundCoin, amount: refund.amount }],
            amount: formatNumber(formatUnits(amount, base.decimals), formatNumberOptions),
          },
        })
      }
    >
      <p className="flex items-center gap-2 diatype-m-medium text-secondary-700">
        {m["notifications.notification.orderCanceled.title"]()}
      </p>

      <div className={twMerge("flex-wrap flex items-center gap-1")}>
        <span>{m["dex.protrade.orderType"]({ orderType: kind })}</span>
        <span
          className={twMerge(
            "uppercase diatype-m-bold",
            direction === Direction.Buy ? "text-status-success" : "text-status-fail",
          )}
        >
          {m["dex.protrade.spot.direction"]({ direction })}
        </span>
        <PairAssets
          assets={[base, quote]}
          className="w-5 h-5 min-w-5 min-h-5"
          mL={(i) => `${-i / 2}rem`}
        />
        <span className="diatype-m-bold">
          {base.symbol}-{quote.symbol}
        </span>
        {limitPrice ? (
          <>
            <span>{m["notifications.notification.orderCreated.atPrice"]()}</span>
            <span className="diatype-m-bold">
              {limitPrice} {quote.symbol}
            </span>
          </>
        ) : null}
      </div>
    </OrderNotification>
  );
};
