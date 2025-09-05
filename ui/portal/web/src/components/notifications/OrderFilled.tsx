import { useConfig } from "@left-curve/store";
import { Direction, OrderType } from "@left-curve/dango/types";

import { OrderNotification } from "./OrderNotification";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { twMerge, useApp } from "@left-curve/foundation";
import { Decimal, formatNumber } from "@left-curve/dango/utils";
import { PairAssets } from "@left-curve/applets-kit";

import type { Notification } from "~/hooks/useNotifications";
import type React from "react";

type NotificationOrderFilledProps = {
  notification: Notification<"orderFilled">;
};

export const NotificationOrderFilled: React.FC<NotificationOrderFilledProps> = ({
  notification,
}) => {
  const { getCoinInfo } = useConfig();
  const { blockHeight, txHash } = notification;
  const { quote_denom, base_denom, clearing_price, remaining, kind, direction, cleared } =
    notification.data;
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const isLimit = kind === OrderType.Limit;

  const base = getCoinInfo(base_denom);
  const quote = getCoinInfo(quote_denom);

  const at = isLimit
    ? formatNumber(
        Decimal(clearing_price)
          .times(Decimal(10).pow(base.decimals - quote.decimals))
          .toFixed(),
        { ...formatNumberOptions, minSignificantDigits: 8, maxSignificantDigits: 8 },
      ).slice(0, 7)
    : null;

  const width = cleared
    ? null
    : formatNumber(remaining, {
        ...formatNumberOptions,
        minSignificantDigits: 8,
        maxSignificantDigits: 8,
      }).slice(0, 7);

  return (
    <OrderNotification kind={kind} txHash={txHash} blockHeight={blockHeight}>
      <p className="flex items-center gap-2 diatype-m-medium text-secondary-700">
        {m["notifications.notification.orderFilled.title"]({
          isFullfilled: m["notifications.notification.orderFilled.isFullfilled"]({
            isFullfilled: String(cleared),
          }),
        })}
      </p>

      <div className={twMerge("flex-wrap flex items-center gap-1")}>
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
        {at ? (
          <>
            <span>{m["notifications.notification.orderCreated.atPrice"]()}</span>
            <span className="diatype-m-bold">
              {at} {quote.symbol}
            </span>
          </>
        ) : null}
        {!cleared ? (
          <>
            <span>{m["common.width"]()}</span>
            <span className="diatype-m-bold">
              {width} {base.symbol}
            </span>
            <span>{m["notifications.notification.orderFilled.remaining"]()}</span>
          </>
        ) : null}
      </div>
    </OrderNotification>
  );
};
