import { forwardRef, useImperativeHandle } from "react";
import { twMerge } from "@left-curve/foundation";
import { formatNumber } from "@left-curve/dango/utils";
import { useApp } from "@left-curve/foundation";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { OrderActivity } from "./OrderActivity";

import type { ActivityRef } from "./Activity";
import type { ActivityRecord } from "@left-curve/store";

type ActivityPerpDeleveragedProps = {
  activity: ActivityRecord<"perpDeleveraged">;
};

export const ActivityPerpDeleveraged = forwardRef<ActivityRef, ActivityPerpDeleveragedProps>(
  ({ activity }, ref) => {
    const { settings } = useApp();
    const { formatNumberOptions } = settings;
    const { pair_id, closing_size, fill_price, realized_pnl } = activity.data;

    const absSize = closing_size.startsWith("-") ? closing_size.slice(1) : closing_size;
    const baseSymbol = pair_id.replace("perp/", "").replace("usd", "").toUpperCase();
    const pairLabel = `${baseSymbol}/USD`;

    useImperativeHandle(ref, () => ({
      onClick: () => {},
    }));

    return (
      <OrderActivity kind="market">
        <p className="flex items-center gap-2 diatype-m-medium text-ink-secondary-700">
          {m["activities.activity.perpDeleveraged.title"]()}
        </p>

        <div className="flex flex-col items-start">
          <div className="flex flex-col gap-1 text-ink-tertiary-500">
            <div className="flex w-full gap-1">
              <span>{pairLabel}</span>
              <span className="diatype-m-bold">
                {formatNumber(absSize, formatNumberOptions)} {baseSymbol}
              </span>
            </div>

            <div className="flex w-full gap-1">
              <span>{m["activities.activity.perpOrderFilled.atPrice"]()}</span>
              <span className="diatype-m-bold">
                ${formatNumber(fill_price, formatNumberOptions)}
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
                  {formatNumber(realized_pnl, formatNumberOptions)}
                </span>
              </div>
            )}
          </div>
        </div>
      </OrderActivity>
    );
  },
);
