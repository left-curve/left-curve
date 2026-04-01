import { forwardRef, useImperativeHandle } from "react";
import { twMerge } from "@left-curve/foundation";
import { formatNumber } from "@left-curve/dango/utils";
import { useApp } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { OrderActivity } from "./OrderActivity";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";

type ActivityPerpLiquidatedProps = {
  activity: ActivityRecord<"perpLiquidated">;
};

export const ActivityPerpLiquidated = forwardRef<ActivityRef, ActivityPerpLiquidatedProps>(
  ({ activity }, ref) => {
    const { settings } = useApp();
    const { formatNumberOptions } = settings;
    const { pair_id, adl_size, adl_price, adl_realized_pnl } = activity.data;

    const absSize = adl_size.startsWith("-") ? adl_size.slice(1) : adl_size;
    const baseSymbol = pair_id.replace("perp/", "").replace("usd", "").toUpperCase();
    const pairLabel = `${baseSymbol}/USD`;

    useImperativeHandle(ref, () => ({
      onClick: () => {},
    }));

    return (
      <OrderActivity kind="market">
        <p className="flex items-center gap-2 diatype-m-medium text-status-fail">
          {m["activities.activity.perpLiquidated.title"]()}
        </p>

        <div className="flex flex-col items-start">
          <div className="flex flex-col gap-1 text-ink-tertiary-500">
            <div className="flex w-full gap-1">
              <span>{pairLabel}</span>
              <span className="diatype-m-bold">
                {formatNumber(absSize, formatNumberOptions)} {baseSymbol}
              </span>
            </div>

            {adl_price && (
              <div className="flex w-full gap-1">
                <span>{m["activities.activity.perpOrderFilled.atPrice"]()}</span>
                <span className="diatype-m-bold">
                  ${formatNumber(adl_price, formatNumberOptions)}
                </span>
              </div>
            )}

            {adl_realized_pnl !== "0" && (
              <div className="flex w-full gap-1">
                <span>PnL</span>
                <span
                  className={twMerge(
                    "diatype-m-bold",
                    adl_realized_pnl.startsWith("-") ? "text-status-fail" : "text-status-success",
                  )}
                >
                  {!adl_realized_pnl.startsWith("-") ? "+" : ""}
                  {formatNumber(adl_realized_pnl, formatNumberOptions)}
                </span>
              </div>
            )}
          </div>
        </div>
      </OrderActivity>
    );
  },
);
