import type * as TV from "@left-curve/tradingview";
import { Direction } from "@left-curve/dango/types";
import { Decimal, adjustPrice } from "@left-curve/dango/utils";

import type { AnyCoin } from "@left-curve/store/types";
import type {
  OrdersByUserResponse,
  PerpsPositionExtended,
  PerpsOrdersByUserResponse,
  WithId,
} from "@left-curve/dango/types";

type ChartLine = {
  price: number;
  color: string;
  linestyle: number;
};

const COLORS = {
  buy: "#27AE60",
  sell: "#EB5757",
  liq: "#EB5757",
} as const;

export function buildPositionLines(position: PerpsPositionExtended): ChartLine[] {
  const isLong = Decimal(position.size).gt(0);

  const lines: ChartLine[] = [
    {
      price: +Decimal(position.entryPrice).toFixed(),
      color: isLong ? COLORS.buy : COLORS.sell,
      linestyle: 0,
    },
  ];

  if (position.liquidationPrice) {
    lines.push({
      price: +Decimal(position.liquidationPrice).toFixed(),
      color: COLORS.liq,
      linestyle: 1,
    });
  }

  const tp = isLong ? position.conditionalOrderAbove : position.conditionalOrderBelow;
  const sl = isLong ? position.conditionalOrderBelow : position.conditionalOrderAbove;

  if (tp) {
    lines.push({
      price: +Decimal(tp.triggerPrice).toFixed(),
      color: COLORS.buy,
      linestyle: 2,
    });
  }

  if (sl) {
    lines.push({
      price: +Decimal(sl.triggerPrice).toFixed(),
      color: COLORS.sell,
      linestyle: 2,
    });
  }

  return lines;
}

export function buildPerpsOrderLines(
  orders: PerpsOrdersByUserResponse,
  pairId: string,
): ChartLine[] {
  return Object.values(orders)
    .filter((order) => order.pairId === pairId)
    .map((order) => {
      const isBuy = Decimal(order.size).gt(0);
      return {
        price: +Decimal(order.limitPrice).toFixed(),
        color: isBuy ? COLORS.buy : COLORS.sell,
        linestyle: 2,
      };
    });
}

export function buildSpotOrderLines(
  orders: WithId<OrdersByUserResponse>[],
  base: AnyCoin,
  quote: AnyCoin,
): ChartLine[] {
  return orders.map((order) => ({
    price: +adjustPrice(
      +Decimal(order.price)
        .times(Decimal(10).pow(base.decimals - quote.decimals))
        .toFixed(),
    ),
    color: order.direction === Direction.Buy ? COLORS.buy : COLORS.sell,
    linestyle: 2,
  }));
}

export const drawLines = (() => {
  let pending = Promise.resolve();

  return (chart: TV.IChartWidgetApi, lines: ChartLine[]) => {
    pending = pending.then(async () => {
      for (const { id } of chart.getAllShapes()) {
        try {
          if (!chart.getShapeById(id).isSavingEnabled()) {
            chart.removeEntity(id);
          }
        } catch {}
      }

      for (const { price, color, linestyle } of lines) {
        await chart.createShape(
          { price, time: Date.now() },
          {
            shape: "horizontal_line",
            lock: true,
            disableSelection: true,
            disableSave: true,
            overrides: {
              showLabel: false,
              showPrice: true,
              linecolor: color,
              linestyle,
              linewidth: 1,
            },
          },
        );
      }
    });
  };
})();
