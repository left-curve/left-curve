import { forwardRef, useImperativeHandle } from "react";
import { View } from "react-native";
import { useRouter } from "expo-router";

import { useConfig } from "@left-curve/store";
import { Modals, useApp, twMerge } from "@left-curve/foundation";
import { Direction, OrderType, TimeInForceOption, type OrderTypes } from "@left-curve/dango/types";
import { calculatePrice, Decimal, formatNumber, formatUnits } from "@left-curve/dango/utils";
import { OrderActivity } from "./OrderActivity";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";
import { GlobalText } from "../GlobalText";
import { PairAssets } from "../PairAssets";

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

    const router = useRouter();
    const { settings, showModal } = useApp();
    const { formatNumberOptions } = settings;

    const kind: OrderTypes =
      time_in_force === TimeInForceOption.GoodTilCanceled ? OrderType.Limit : OrderType.Market;
    const isLimit = kind === OrderType.Limit;

    const base = getCoinInfo(base_denom);
    const quote = getCoinInfo(quote_denom);
    const refundCoin = getCoinInfo(refund.denom);
    const directionAsk = direction === Direction.Sell /*  || direction === "ask" */;
    const directionBid = direction === Direction.Buy /*  || direction === "bid" */;

    const limitPrice = isLimit
      ? calculatePrice(price, { base: base.decimals, quote: quote.decimals }, formatNumberOptions)
      : null;

    const filled = Decimal(amount).minus(remaining).toFixed();

    useImperativeHandle(ref, () => ({
      onPress: () =>
        /* showModal(Modals.ActivitySpotOrder, {
          base,
          quote,
          blockHeight,
          action: directionAsk ? "sell" : "buy",
          status: "canceled",
          order: {
            id,
            limitPrice,
            type: kind === OrderType.Limit ? "limit" : "market",
            timeCanceled: createdAt,
            filledAmount: formatNumber(formatUnits(filled, base.decimals), formatNumberOptions),
            refund: [{ ...refundCoin, amount: refund.amount }],
            amount: formatNumber(formatUnits(amount, base.decimals), formatNumberOptions),
          },
          navigate: (to: string) => router.push(to as any),
        }) */ console.log("Open activitySpotOrder modal"),
    }));

    return (
      <OrderActivity
        kind={kind}
        onClick={() =>
          showModal(Modals.ActivitySpotOrder, {
            base,
            quote,
            blockHeight,
            action: directionAsk ? "sell" : "buy",
            status: "canceled",
            order: {
              id,
              limitPrice,
              type: kind === OrderType.Limit ? "limit" : "market",
              timeCanceled: createdAt,
              filledAmount: formatNumber(formatUnits(filled, base.decimals), formatNumberOptions),
              refund: [{ ...refundCoin, amount: refund.amount }],
              amount: formatNumber(formatUnits(amount, base.decimals), formatNumberOptions),
            },
            navigate: (to: string) => router.push(to as any),
          })
        }
      >
        <GlobalText className="flex flex-row items-center gap-2 diatype-m-medium text-ink-secondary-700">
          {m["activities.activity.orderCanceled.title"]()}
        </GlobalText>

        <View className="flex flex-col items-start">
          <View className="flex flex-row gap-1 items-center">
            <GlobalText>{m["dex.protrade.orderType"]({ orderType: kind })}</GlobalText>

            <GlobalText
              className={twMerge(
                "uppercase diatype-m-bold",
                directionBid ? "text-status-success" : "text-status-fail",
              )}
            >
              {directionBid ? m["proSwap.buy"]() : m["proSwap.sell"]()}
            </GlobalText>

            <PairAssets assets={[base, quote]} className="w-5 h-5 min-w-5 min-h-5" />

            <GlobalText className="diatype-m-bold">
              {base.symbol}-{quote.symbol}
            </GlobalText>
          </View>

          {limitPrice ? (
            <View className="flex flex-row gap-1 items-center">
              <GlobalText>{m["activities.activity.orderCanceled.atPrice"]()}</GlobalText>
              <GlobalText className="diatype-m-bold">
                {limitPrice} {quote.symbol}
              </GlobalText>
            </View>
          ) : null}
        </View>
      </OrderActivity>
    );
  },
);
