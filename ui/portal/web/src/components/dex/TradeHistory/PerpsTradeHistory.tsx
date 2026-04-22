import { Cell, FormattedNumber } from "@left-curve/applets-kit";
import { usePublicClient, useAccount, useQueryWithPagination } from "@left-curve/store";
import { Decimal } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { TradeHistoryTable } from "./TradeHistoryTable";

import type { TableColumn } from "@left-curve/applets-kit";
import type {
  PerpsEvent,
  OrderFilledData,
  LiquidatedData,
  DeleveragedData,
} from "@left-curve/dango/types";

const eventTypeLabels: Record<string, string> = {
  order_filled: "Trade",
  liquidated: "Liquidation",
  deleveraged: "ADL",
};

function getPerpsEventSize(eventType: string, data: PerpsEvent["data"]): string | null {
  if (eventType === "order_filled") return (data as OrderFilledData).fill_size;
  if (eventType === "liquidated") return (data as LiquidatedData).adl_size;
  if (eventType === "deleveraged") return (data as DeleveragedData).closing_size;
  return null;
}

function getPerpsEventPrice(eventType: string, data: PerpsEvent["data"]): string | null {
  if (eventType === "order_filled") return (data as OrderFilledData).fill_price;
  if (eventType === "liquidated") return (data as LiquidatedData).adl_price;
  if (eventType === "deleveraged") return (data as DeleveragedData).fill_price;
  return null;
}

function getPerpsEventPnl(eventType: string, data: PerpsEvent["data"]): string | null {
  if (eventType === "order_filled") return (data as OrderFilledData).realized_pnl;
  if (eventType === "liquidated") return (data as LiquidatedData).adl_realized_pnl;
  if (eventType === "deleveraged") return (data as DeleveragedData).realized_pnl;
  return null;
}

function getPerpsEventFee(eventType: string, data: PerpsEvent["data"]): string | null {
  if (eventType === "order_filled") return (data as OrderFilledData).fee;
  return null;
}

export const PerpsTradeHistory: React.FC = () => {
  const { account } = useAccount();
  const publicClient = usePublicClient();
  const { data, pagination, isLoading } = useQueryWithPagination({
    enabled: !!account,
    queryKey: ["perpsTradeHistory", account?.address as string],
    queryFn: async () => {
      if (!account) throw new Error();
      return await publicClient.queryPerpsEvents({
        userAddr: account.address,
        sortBy: "BLOCK_HEIGHT_DESC",
      });
    },
  });

  const columns: TableColumn<PerpsEvent> = [
    {
      header: m["dex.protrade.tradeHistory.pair"](),
      cell: ({ row }) => {
        const pair = row.original.pairId.replace("perp/", "").replace("usd", "/USD").toUpperCase();
        return <Cell.Text text={pair} className="diatype-xs-medium" />;
      },
    },
    {
      header: m["dex.protrade.history.type"](),
      cell: ({ row }) => (
        <Cell.Text text={eventTypeLabels[row.original.eventType] ?? row.original.eventType} />
      ),
    },
    {
      header: "Direction",
      cell: ({ row }) => {
        const size = getPerpsEventSize(row.original.eventType, row.original.data);
        if (!size) return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        const isBuy = !size.startsWith("-");
        return (
          <Cell.Text
            text={isBuy ? "Buy" : "Sell"}
            className={isBuy ? "text-green-500" : "text-red-500"}
          />
        );
      },
    },
    {
      header: "Size",
      cell: ({ row }) => {
        const size = getPerpsEventSize(row.original.eventType, row.original.data);
        if (!size) return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        const abs = size.startsWith("-") ? size.slice(1) : size;
        const baseSymbol = row.original.pairId.replace("perp/", "").replace("usd", "").toUpperCase();
        return (
          <Cell.Text
            text={<><FormattedNumber number={abs} as="span" /> {baseSymbol}</>}
          />
        );
      },
    },
    {
      header: m["dex.protrade.tradeHistory.tradeValue"](),
      cell: ({ row }) => {
        const size = getPerpsEventSize(row.original.eventType, row.original.data);
        const price = getPerpsEventPrice(row.original.eventType, row.original.data);
        if (!size || !price) return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        const absSize = size.startsWith("-") ? size.slice(1) : size;
        const orderValue = Decimal(absSize).times(Decimal(price)).toFixed();
        return (
          <Cell.Text
            text={<FormattedNumber number={orderValue} formatOptions={{ currency: "USD" }} as="span" />}
          />
        );
      },
    },
    {
      header: m["dex.protrade.history.price"](),
      cell: ({ row }) => {
        const price = getPerpsEventPrice(row.original.eventType, row.original.data);
        if (!price) return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        return <Cell.Text text={<FormattedNumber number={price} formatOptions={{ currency: "USD" }} as="span" />} />;
      },
    },
    {
      header: "PnL",
      cell: ({ row }) => {
        const pnl = getPerpsEventPnl(row.original.eventType, row.original.data);
        if (!pnl || pnl === "0") return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        const isPositive = !pnl.startsWith("-");
        return (
          <Cell.Text
            text={<>{isPositive ? "+" : ""}<FormattedNumber number={pnl} as="span" /></>}
            className={isPositive ? "text-green-500" : "text-red-500"}
          />
        );
      },
    },
    {
      header: "Fees",
      cell: ({ row }) => {
        const fee = getPerpsEventFee(row.original.eventType, row.original.data);
        if (!fee || fee === "0") return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        return (
          <Cell.Text
            text={<FormattedNumber number={fee} formatOptions={{ currency: "USD" }} as="span" />}
          />
        );
      },
    },
    {
      header: "Time",
      cell: ({ row }) => <Cell.Time date={row.original.createdAt} dateFormat="MM/dd/yy h:mm a" />,
    },
  ];

  return (
    <TradeHistoryTable data={data} columns={columns} isLoading={isLoading} />
  );
};
