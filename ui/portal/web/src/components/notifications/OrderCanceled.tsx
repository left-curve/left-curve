import { useConfig } from "@left-curve/store";

import { Decimal, formatNumber, formatUnits } from "@left-curve/dango/utils";
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
  const { createdAt } = notification;
  const { id, quote_denom, base_denom, price, kind, direction, amount, refund, remaining } =
    notification.data;
  const { settings, showModal } = useApp();
  const { formatNumberOptions } = settings;

  const isLimit = kind === OrderType.Limit;

  const base = getCoinInfo(base_denom);
  const quote = getCoinInfo(quote_denom);
  const refundCoins = getCoinInfo(refund.denom);

  const at = isLimit
    ? formatNumber(
        Decimal(price)
          .times(Decimal(10).pow(base.decimals - quote.decimals))
          .toFixed(),
        { ...formatNumberOptions, minSignificantDigits: 8, maxSignificantDigits: 8 },
      ).slice(0, 7)
    : null;

  const filled = Decimal(amount).minus(remaining).toFixed();

  return (
    <OrderNotification
      kind={kind}
      onClick={() =>
        showModal("notification-spot-action-order", {
          base,
          quote,
          action: direction === "ask" ? "sell" : "buy",
          status: "canceled",
          order: {
            id,
            type: kind,
            timeCanceled: createdAt,
            filledAmount: formatNumber(formatUnits(filled, base.decimals), {
              ...formatNumberOptions,
            }),
            tokenReceived: `${formatNumber(formatUnits(refund.amount, refundCoins.decimals), {
              ...formatNumberOptions,
            })} ${refundCoins.symbol}`,
            limitPrice: at,
            amount: formatNumber(formatUnits(amount, base.decimals), {
              ...formatNumberOptions,
            }),
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
        {at ? (
          <>
            <span>{m["notifications.notification.orderCreated.atPrice"]()}</span>
            <span className="diatype-m-bold">
              {at} {quote.symbol}
            </span>
          </>
        ) : null}
      </div>
    </OrderNotification>
  );
};
