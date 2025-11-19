import { useConfig } from "@left-curve/store";

import { forwardRef, useImperativeHandle } from "react";

import { twMerge, useApp } from "@left-curve/foundation";
import { Direction, OrderType, TimeInForceOption } from "@left-curve/dango/types";
import { calculatePrice } from "@left-curve/dango/utils";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";
import { OrderActivity } from "./OrderActivity";
import { PairAssets } from "../PairAssets";
import { GlobalText } from "../GlobalText";
import { View } from "react-native";

type ActivityOrderCreatedProps = {
  activity: ActivityRecord<"orderCreated">;
};

export const ActivityOrderCreated = forwardRef<ActivityRef, ActivityOrderCreatedProps>(
  ({ activity }, ref) => {
    const { getCoinInfo } = useConfig();
    const { settings } = useApp();
    const { quote_denom, base_denom, price, time_in_force, direction } = activity.data;
    const { formatNumberOptions } = settings;

    const kind = time_in_force === TimeInForceOption.GoodTilCanceled ? "limit" : "market";
    const isLimit = kind === OrderType.Limit;

    const base = getCoinInfo(base_denom);
    const quote = getCoinInfo(quote_denom);

    const directionAsk = direction === Direction.Sell /*  || direction === "ask" */;
    const directionBid = direction === Direction.Buy /*  || direction === "bid" */;

    const limitPrice = isLimit
      ? calculatePrice(price, { base: base.decimals, quote: quote.decimals }, formatNumberOptions)
      : null;

    useImperativeHandle(ref, () => ({
      onPress: () =>
        /*  showModal(Modals.ActivitySpotOrder, {
          base,
          quote,
          blockHeight,
          action: directionAsk ? "sell" : "buy",
          status: "created",
          order: {
            id,
            type: kind,
            timeCreated: createdAt,
            limitPrice,
            amount: formatNumber(formatUnits(amount, base.decimals), formatNumberOptions),
          },
          navigate,
        }) */ console.log("show ActivitySpotOrder modal"),
    }));

    return (
      <OrderActivity kind={kind}>
        <GlobalText className="flex items-center gap-2 diatype-m-medium text-ink-secondary-700">
          Order created
        </GlobalText>

        <View className="flex flex-col items-start">
          <View className="flex gap-1">
            <GlobalText className="capitalize">{kind}</GlobalText>
            <GlobalText
              className={twMerge(
                "uppercase diatype-m-bold",
                directionBid ? "text-status-success" : "text-status-fail",
              )}
            >
              {directionAsk ? "Sell" : "Buy"}
            </GlobalText>
            <PairAssets assets={[base, quote]} className="w-5 h-5 min-w-5 min-h-5" />
            <GlobalText className="diatype-m-bold">
              {base.symbol}-{quote.symbol}
            </GlobalText>
          </View>
          {limitPrice ? (
            <View className="flex gap-1">
              <GlobalText>at price </GlobalText>
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
