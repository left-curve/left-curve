import { useConfig } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { useApp } from "~/hooks/useApp";

import { m } from "~/paraglide/messages";
import { Decimal, formatNumber } from "@left-curve/dango/utils";
import { OrderType } from "@left-curve/dango/types";

import {
  Badge,
  IconLimitOrder,
  IconMarketOrder,
  PairAssets,
  twMerge,
} from "@left-curve/applets-kit";

import type { Notification } from "~/hooks/useNotifications";
import type React from "react";

type NotificationOrderCreatedProps = {
  notification: Notification<"orderCreated">;
};

export const NotificationOrderCreated: React.FC<NotificationOrderCreatedProps> = ({
  notification,
}) => {
  const navigate = useNavigate();
  const { settings, setNotificationMenuVisibility } = useApp();
  const { getCoinInfo } = useConfig();
  const { blockHeight, txHash } = notification;
  const { quote_denom, base_denom, price, kind, deposit: depositInfo } = notification.data;
  const { formatNumberOptions } = settings;

  const base = getCoinInfo(base_denom);
  const quote = getCoinInfo(quote_denom);
  const deposit = getCoinInfo(depositInfo.denom);

  const at = formatNumber(
    Decimal(price)
      .times(Decimal(10).pow(base.decimals - quote.decimals))
      .toFixed(),
    { ...formatNumberOptions, minSignificantDigits: 8, maxSignificantDigits: 8 },
  ).slice(0, 7);

  const width = formatNumber(
    Decimal(depositInfo.amount).div(Decimal(10).pow(deposit.decimals)).toFixed(),
    { ...formatNumberOptions, minSignificantDigits: 8, maxSignificantDigits: 8 },
  ).slice(0, 7);

  const isLimit = kind === OrderType.Limit;

  const Icon = isLimit ? IconLimitOrder : IconMarketOrder;

  const onNavigate = (url: string) => {
    setNotificationMenuVisibility(false);
    navigate({ to: url });
  };
  return (
    <div
      className="flex items-start gap-2 max-w-full overflow-hidden cursor-pointer"
      onClick={(event) => {
        const element = event.target as HTMLElement;
        if (element.closest(".address-visualizer") || element.closest(".remove-notification")) {
          return;
        }
        onNavigate(txHash ? `/tx/${txHash}` : `/block/${blockHeight}`);
      }}
    >
      <div
        className={twMerge(
          "flex items-center justify-center min-w-7 min-h-7 w-7 h-7 rounded-sm",
          isLimit ? "bg-blue-200" : "bg-tertiary-green",
        )}
      >
        <Icon className={twMerge(isLimit ? "text-secondary-blue" : "text-brand-green")} />
      </div>

      <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
        <div className="flex items-center gap-2 diatype-m-medium text-secondary-700 capitalize">
          <span>{m["notifications.notification.orderCreated.title"]({ orderType: kind })}</span>
          <Badge text="Spot" />
        </div>

        <div className={twMerge("flex-wrap flex items-center gap-1")}>
          <span>{m["common.for"]()}</span>
          <PairAssets
            assets={[base, quote]}
            className="w-5 h-5 min-w-5 min-h-5"
            mL={(i) => `${-i / 2}rem`}
          />
          <span className="diatype-m-bold">
            {base.symbol}-{quote.symbol}
          </span>
          <span>{m["common.at"]()}</span>
          <span className="diatype-m-bold">
            {at} {quote.symbol}
          </span>
          <span>{m["common.width"]()}</span>
          <span className="diatype-m-bold">
            {width} {deposit.symbol}
          </span>
        </div>
      </div>
    </div>
  );
};
