import { useApp } from "@left-curve/foundation";
import { forwardRef, useImperativeHandle } from "react";
import { useConfig } from "@left-curve/store";

import { Direction, TimeInForceOption } from "@left-curve/dango/types";
import { twMerge } from "@left-curve/foundation";
import { formatNumber } from "@left-curve/dango/utils";

import { OrderActivity } from "./OrderActivity";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";
import { PairAssets } from "../PairAssets";
import { GlobalText } from "../GlobalText";

type ActivityOrderFilledProps = {
  activity: ActivityRecord<"orderFilled">;
};

export const ActivityOrderFilled = forwardRef<ActivityRef, ActivityOrderFilledProps>(
  ({ activity }, ref) => {
    const { getCoinInfo } = useConfig();
    const { quote_denom, base_denom, remaining, time_in_force, direction, cleared } = activity.data;
    const { settings } = useApp();
    const { formatNumberOptions } = settings;
    /* const { createdAt, blockHeight } = activity;
    const { navigate } = useRouter();
    const { getPrice } = usePrices(); */

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

    const width = cleared ? null : formatNumber(remaining, formatNumberOptions);

    /*  const filled =
      direction === Direction.Buy
        ? filled_base
        : Decimal(filled_quote).div(clearing_price).toFixed(); */

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
        <p className="flex items-center gap-2 diatype-m-medium text-ink-secondary-700">
          Order {cleared ? "filled" : "partially fulfilled"}
        </p>

        <div className="flex flex-col items-start">
          <div className="flex gap-1">
            <GlobalText>{kind}</GlobalText>
            <GlobalText
              className={twMerge(
                "uppercase diatype-m-bold",
                direction === Direction.Buy ? "text-status-success" : "text-status-fail",
              )}
            >
              {direction === Direction.Buy ? "Buy" : "Sell"}
            </GlobalText>
            <PairAssets assets={[base, quote]} className="w-5 h-5 min-w-5 min-h-5" />
            <GlobalText className="diatype-m-bold">
              {base.symbol}-{quote.symbol}
            </GlobalText>
            {limitPrice ? (
              <>
                <GlobalText>at price</GlobalText>
                <GlobalText className="diatype-m-bold">
                  {limitPrice} {quote.symbol}
                </GlobalText>
              </>
            ) : null}
          </div>
          {!cleared ? (
            <div className="flex gap-1">
              <GlobalText>width</GlobalText>
              <GlobalText className="diatype-m-bold">
                {width} {base.symbol}
              </GlobalText>
              <GlobalText>remaining</GlobalText>
            </div>
          ) : null}
        </div>
      </OrderActivity>
    );
  },
);
