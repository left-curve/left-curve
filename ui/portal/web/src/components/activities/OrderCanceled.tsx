import { useConfig } from "@left-curve/store";
import { useApp } from "@left-curve/applets-kit";
import { useRouter } from "@tanstack/react-router";
import { forwardRef, useImperativeHandle } from "react";

import { OrderActivity } from "./OrderActivity";
import { PairAssets, twMerge } from "@left-curve/applets-kit";
import { Direction, OrderType, TimeInForceOption } from "@left-curve/dango/types";

import { calculatePrice, Decimal, formatNumber, formatUnits } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";

type ActivityOrderCanceledProps = {
  activity: ActivityRecord<"orderCanceled">;
};

export const ActivityOrderCanceled = forwardRef<ActivityRef, ActivityOrderCanceledProps>(
  ({ activity }, ref) => {
    const { getCoinInfo } = useConfig();
    const { createdAt, blockHeight } = activity;
    const {
      id,
      quote_denom,
      base_denom,
      price,
      time_in_force,
      direction,
      amount,
      refund,
      remaining,
    } = activity.data;
    const { navigate } = useRouter();
    const { settings, showModal } = useApp();
    const { formatNumberOptions } = settings;

    const kind = time_in_force === TimeInForceOption.GoodTilCanceled ? "limit" : "market";
    const isLimit = kind === OrderType.Limit;

    const base = getCoinInfo(base_denom);
    const quote = getCoinInfo(quote_denom);
    const refundCoin = getCoinInfo(refund.denom);

    const limitPrice = isLimit
      ? calculatePrice(price, { base: base.decimals, quote: quote.decimals }, formatNumberOptions)
      : null;

    const filled = Decimal(amount).minus(remaining).toFixed();

    useImperativeHandle(ref, () => ({
      onClick: () =>
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
          navigate,
        }),
    }));

    return (
      <OrderActivity kind={kind}>
        <p className="flex items-center gap-2 diatype-m-medium text-secondary-700">
          {m["notifications.notification.orderCanceled.title"]()}
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
      </OrderActivity>
    );
  },
);
