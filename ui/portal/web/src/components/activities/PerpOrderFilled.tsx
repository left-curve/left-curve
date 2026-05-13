import { forwardRef, useImperativeHandle } from "react";
import { twMerge } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { FormattedNumber } from "@left-curve/applets-kit";

import { OrderActivity } from "./OrderActivity";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";

type ActivityPerpOrderFilledProps = {
  activity: ActivityRecord<"perpOrderFilled">;
};

export const ActivityPerpOrderFilled = forwardRef<ActivityRef, ActivityPerpOrderFilledProps>(
  ({ activity }, ref) => {
    const { pair_id, fill_price, fill_size, realized_pnl, fee, is_maker } = activity.data;

    const isBuy = !fill_size.startsWith("-");
    const absSize = fill_size.startsWith("-") ? fill_size.slice(1) : fill_size;
    const baseSymbol = pair_id.replace("perp/", "").replace("usd", "").toUpperCase();
    const pairLabel = `${baseSymbol}/USD`;

    useImperativeHandle(ref, () => ({
      onClick: () => {},
    }));

    return (
      <OrderActivity kind="market">
        <p className="flex items-center gap-2 diatype-m-medium text-ink-secondary-700">
          {m["activities.activity.perpOrderFilled.title"]()}
        </p>

        <div className="flex flex-col items-start">
          <div className="flex flex-col gap-1 text-ink-tertiary-500">
            <div className="flex w-full gap-1">
              <span>{pairLabel}</span>
              <span
                className={twMerge(
                  "uppercase diatype-m-bold",
                  isBuy ? "text-status-success" : "text-status-fail",
                )}
              >
                {isBuy ? "Long" : "Short"}
              </span>
              <span className="diatype-m-bold">
                <FormattedNumber number={absSize} as="span" /> {baseSymbol}
              </span>
              {is_maker != null && (
                <span className="uppercase diatype-m-bold text-ink-tertiary-500">
                  {is_maker ? "Maker" : "Taker"}
                </span>
              )}
            </div>

            <div className="flex w-full gap-1">
              <span>{m["activities.activity.perpOrderFilled.atPrice"]()}</span>
              <span className="diatype-m-bold">
                <FormattedNumber number={fill_price} formatOptions={{ currency: "USD" }} as="span" />
              </span>
            </div>

            {realized_pnl !== "0" && (
              <div className="flex w-full gap-1">
                <span>PnL</span>
                <span
                  className={twMerge(
                    "diatype-m-bold",
                    realized_pnl.startsWith("-") ? "text-status-fail" : "text-status-success",
                  )}
                >
                  {!realized_pnl.startsWith("-") ? "+" : ""}
                  <FormattedNumber number={realized_pnl} as="span" />
                </span>
              </div>
            )}
          </div>
        </div>
      </OrderActivity>
    );
  },
);
