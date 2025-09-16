import { Modals, useApp } from "@left-curve/foundation";
import { useRouter } from "@tanstack/react-router";
import { forwardRef, useImperativeHandle } from "react";
import { useConfig, usePrices } from "@left-curve/store";

import { Direction, TimeInForceOption } from "@left-curve/dango/types";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { twMerge } from "@left-curve/foundation";
import {
  calculateFees,
  calculatePrice,
  Decimal,
  formatNumber,
  formatUnits,
} from "@left-curve/dango/utils";
import { PairAssets } from "@left-curve/applets-kit";

import { OrderActivity } from "./OrderActivity";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";

type ActivityOrderFilledProps = {
  activity: ActivityRecord<"orderFilled">;
};

export const ActivityOrderFilled = forwardRef<ActivityRef, ActivityOrderFilledProps>(
  ({ activity }, ref) => {
    const { getCoinInfo } = useConfig();
    const { createdAt, blockHeight } = activity;
    const {
      id,
      quote_denom,
      base_denom,
      clearing_price,
      remaining,
      time_in_force,
      direction,
      cleared,
      fee_base,
      fee_quote,
      filled_base,
      filled_quote,
      refund_base,
      refund_quote,
    } = activity.data;
    const { settings, showModal } = useApp();
    const { navigate } = useRouter();
    const { formatNumberOptions } = settings;
    const { getPrice } = usePrices();

    const kind = time_in_force === TimeInForceOption.GoodTilCanceled ? "limit" : "market";

    const base = getCoinInfo(base_denom);
    const quote = getCoinInfo(quote_denom);

    const fee = calculateFees(
      { amount: fee_base, decimals: base.decimals, price: getPrice(1, base.denom) },
      { amount: fee_quote, decimals: quote.decimals, price: getPrice(1, quote.denom) },
      formatNumberOptions,
    );

    const averagePrice = calculatePrice(
      clearing_price,
      { base: base.decimals, quote: quote.decimals },
      formatNumberOptions,
    );

    const limitPrice = null;

    const width = cleared
      ? null
      : formatNumber(remaining, {
          ...formatNumberOptions,
          minSignificantDigits: 8,
          maxSignificantDigits: 8,
        }).slice(0, 7);

    const filled =
      direction === Direction.Buy
        ? filled_base
        : Decimal(filled_quote).div(clearing_price).toFixed();

    useImperativeHandle(ref, () => ({
      onClick: () =>
        showModal(Modals.ActivitySpotOrder, {
          base,
          quote,
          blockHeight,
          action: direction === "ask" ? "sell" : "buy",
          status: cleared ? "fulfilled" : "partially fulfilled",
          order: {
            id,
            fee,
            averagePrice,
            type: kind,
            timeCreated: createdAt,
            filled: formatNumber(formatUnits(filled, base.decimals), formatNumberOptions),
            refund: [
              { ...base, amount: refund_base },
              { ...quote, amount: refund_quote },
            ],
          },
          navigate,
        }),
    }));

    return (
      <OrderActivity kind={kind}>
        <p className="flex items-center gap-2 diatype-m-medium text-secondary-700">
          {m["activities.activity.orderFilled.title"]({
            isFullfilled: m["activities.activity.orderFilled.isFullfilled"]({
              isFullfilled: String(cleared),
            }),
          })}
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
            {limitPrice ? (
              <>
                <span>{m["activities.activity.orderCreated.atPrice"]()}</span>
                <span className="diatype-m-bold">
                  {limitPrice} {quote.symbol}
                </span>
              </>
            ) : null}
          </div>
          {!cleared ? (
            <div className="flex gap-1">
              <span>{m["common.width"]()}</span>
              <span className="diatype-m-bold">
                {width} {base.symbol}
              </span>
              <span>{m["activities.activity.orderFilled.remaining"]()}</span>
            </div>
          ) : null}
        </div>
      </OrderActivity>
    );
  },
);
