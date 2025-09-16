import { OrderType, type OrderTypes } from "@left-curve/dango/types";

import { IconLimitOrder, IconMarketOrder, twMerge } from "@left-curve/applets-kit";

import type React from "react";
import type { PropsWithChildren } from "react";

type OrderActivityProps = {
  kind: OrderTypes;
  onClick?: () => void;
};

export const OrderActivity: React.FC<PropsWithChildren<OrderActivityProps>> = (parameters) => {
  const { onClick, kind, children } = parameters;
  const isLimit = kind === OrderType.Limit;
  const Icon = isLimit ? IconLimitOrder : IconMarketOrder;

  return (
    <div
      className="flex items-start gap-2 max-w-full overflow-hidden cursor-pointer"
      onClick={(event) => {
        const element = event.target as HTMLElement;
        if (element.closest(".address-visualizer") || element.closest(".remove-activity")) {
          return;
        }
        onClick?.();
      }}
    >
      <div
        className={twMerge(
          "flex items-center justify-center min-w-7 min-h-7 w-7 h-7 rounded-sm",
          isLimit ? "bg-tertiary-blue" : "bg-tertiary-green",
        )}
      >
        <Icon className={twMerge(isLimit ? "text-secondary-blue" : "text-brand-green")} />
      </div>
      <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">{children}</div>
    </div>
  );
};
