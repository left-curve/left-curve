import { useConfig } from "@left-curve/store";
import { useRouter } from "@tanstack/react-router";

import { forwardRef, useImperativeHandle } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { OrderNotification } from "./OrderNotification";
import { PairAssets, twMerge, useApp } from "@left-curve/applets-kit";
import { Direction, OrderType, TimeInForceOption } from "@left-curve/dango/types";
import { calculatePrice, formatNumber, formatUnits } from "@left-curve/dango/utils";

import type { Notification } from "~/hooks/useNotifications";
import type { NotificationRef } from "./Notification";

type NotificationOrderCreatedProps = {
  notification: Notification<"orderCreated">;
};

export const NotificationOrderCreated = forwardRef<NotificationRef, NotificationOrderCreatedProps>(
  ({ notification }, ref) => {
    const { navigate } = useRouter();
    const { getCoinInfo } = useConfig();
    const { settings, showModal } = useApp();
    const { blockHeight, createdAt } = notification;
    const { id, quote_denom, base_denom, price, time_in_force, direction, amount } =
      notification.data;
    const { formatNumberOptions } = settings;

    const kind = time_in_force === TimeInForceOption.GoodTilCanceled ? "limit" : "market";
    const isLimit = kind === OrderType.Limit;

    const base = getCoinInfo(base_denom);
    const quote = getCoinInfo(quote_denom);

    const limitPrice = isLimit
      ? calculatePrice(price, { base: base.decimals, quote: quote.decimals }, formatNumberOptions)
      : null;

    useImperativeHandle(ref, () => ({
      onClick: () =>
        showModal("notification-spot-action-order", {
          base,
          quote,
          blockHeight,
          action: direction === "ask" ? "sell" : "buy",
          status: "created",
          order: {
            id,
            type: kind,
            timeCreated: createdAt,
            limitPrice,
            amount: formatNumber(formatUnits(amount, base.decimals), formatNumberOptions),
          },
          navigate,
        }),
    }));

    return (
      <OrderNotification kind={kind}>
        <p className="flex items-center gap-2 diatype-m-medium text-secondary-700">
          {m["notifications.notification.orderCreated.title"]()}
        </p>

        <div className="flex flex-col items-start">
          <div className="flex gap-1">
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
          </div>
          {limitPrice ? (
            <div className="flex gap-1">
              <span>{m["notifications.notification.orderCreated.atPrice"]()}</span>
              <span className="diatype-m-bold">
                {limitPrice} {quote.symbol}
              </span>
            </div>
          ) : null}
        </div>
      </OrderNotification>
    );
  },
);
