import { useApp } from "@left-curve/foundation";
import { forwardRef, useImperativeHandle } from "react";
import { useConfig } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { Direction, TimeInForceOption } from "@left-curve/dango/types";
import { twMerge } from "@left-curve/foundation";
import { calculatePrice } from "@left-curve/dango/utils";

import { OrderActivity } from "./OrderActivity";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";
import { PairAssets } from "../PairAssets";
import { GlobalText } from "../GlobalText";
import { View } from "react-native";

type ActivityOrderFilledProps = {
  activity: ActivityRecord<"orderFilled">;
};

export const ActivityOrderFilled = forwardRef<ActivityRef, ActivityOrderFilledProps>(
  ({ activity }, ref) => {
    const { getCoinInfo } = useConfig();
    const { quote_denom, base_denom, clearing_price, time_in_force, direction, cleared } =
      activity.data;
    const { settings } = useApp();
    const { formatNumberOptions } = settings;

    const kind = time_in_force === TimeInForceOption.GoodTilCanceled ? "limit" : "market";

    const base = getCoinInfo(base_denom);
    const quote = getCoinInfo(quote_denom);

    /*  const fee = calculateFees(
      { amount: fee_base, decimals: base.decimals, price: getPrice(1, base.denom) },
      { amount: fee_quote, decimals: quote.decimals, price: getPrice(1, quote.denom) },
      formatNumberOptions,
    ); */

    /* const averagePrice = calculatePrice(
      clearing_price,
      { base: base.decimals, quote: quote.decimals },
      formatNumberOptions,
    ); */

    const limitPrice = null;

    const averagePrice = calculatePrice(
      clearing_price,
      { base: base.decimals, quote: quote.decimals },
      formatNumberOptions,
    );
    useImperativeHandle(ref, () => ({
      onPress: () =>
        /* showModal(Modals.ActivitySpotOrder, {
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
        }) */ console.log("Open activitySpotOrder modal"),
    }));

    return (
      <OrderActivity kind={kind}>
        <GlobalText className="flex items-center gap-2 diatype-m-medium text-ink-secondary-700">
          Order {cleared ? "fulfilled" : "partially fulfilled"}
        </GlobalText>

        <View className="flex flex-col items-start w-full">
          <View className="flex gap-2 w-full flex-row">
            <GlobalText className="capitalize text-ink-tertiary-500">{kind}</GlobalText>
            <GlobalText
              className={twMerge(
                "uppercase diatype-m-bold",
                direction === Direction.Buy ? "text-status-success" : "text-status-fail",
              )}
            >
              {direction === Direction.Buy ? "Buy" : "Sell"}
            </GlobalText>
            <PairAssets assets={[base, quote]} className="w-5 h-5 min-w-5 min-h-5" />
            <GlobalText className="diatype-m-bold text-ink-tertiary-500">
              {base.symbol}-{quote.symbol}
            </GlobalText>
            {limitPrice ? (
              <>
                <GlobalText className="text-ink-tertiary-500">
                  {m["activities.activity.orderCreated.atPrice"]()}
                </GlobalText>
                <GlobalText className="diatype-m-bold text-ink-tertiary-500">
                  {limitPrice} {quote.symbol}
                </GlobalText>
              </>
            ) : null}
          </View>
          {averagePrice ? (
            <View className="flex w-full gap-1 flex-row">
              <GlobalText className="text-ink-tertiary-500">
                {m["activities.activity.orderCreated.atPrice"]()}
              </GlobalText>
              <GlobalText className="diatype-m-bold text-ink-tertiary-500">
                {averagePrice} {quote.symbol}
              </GlobalText>
            </View>
          ) : null}
        </View>
      </OrderActivity>
    );
  },
);
