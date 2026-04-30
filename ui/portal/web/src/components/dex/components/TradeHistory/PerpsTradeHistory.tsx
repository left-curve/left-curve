import { Cell, FormattedNumber, Tooltip } from "@left-curve/applets-kit";
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

const V016_CUTOFF = new Date("2026-04-22T12:00:00Z");
const V017_CUTOFF = new Date("2026-04-30T12:00:00Z");

function normalizePerpsEvent(eventType: string, data: PerpsEvent["data"]) {
  switch (eventType) {
    case "order_filled": {
      const d = data as OrderFilledData;
      return {
        size: d.fill_size,
        price: d.fill_price,
        pnl: d.realized_pnl,
        fee: d.fee,
        funding: d.realized_funding,
        isMaker: d.is_maker,
      };
    }
    case "liquidated": {
      const d = data as LiquidatedData;
      return {
        size: d.adl_size,
        price: d.adl_price,
        pnl: d.adl_realized_pnl,
        fee: null,
        funding: d.adl_realized_funding,
        isMaker: null,
      };
    }
    case "deleveraged": {
      const d = data as DeleveragedData;
      return {
        size: d.closing_size,
        price: d.fill_price,
        pnl: d.realized_pnl,
        fee: null,
        funding: d.realized_funding,
        isMaker: null,
      };
    }
    default:
      return { size: null, price: null, pnl: null, fee: null, funding: null, isMaker: null };
  }
}

type NormalizedPerpsEvent = PerpsEvent & ReturnType<typeof normalizePerpsEvent>;

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

  const normalizedData = data
    ? {
        pageInfo: data.pageInfo,
        edge: [] as { cursor: string; node: NormalizedPerpsEvent }[],
        nodes: data.nodes.map((event) => ({
          ...event,
          ...normalizePerpsEvent(event.eventType, event.data),
        })),
      }
    : undefined;

  const columns: TableColumn<NormalizedPerpsEvent> = [
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
      header: m["dex.protrade.tradeHistory.direction"](),
      cell: ({ row }) => {
        const { size } = row.original;
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
        const { size } = row.original;
        if (!size) return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        const abs = size.startsWith("-") ? size.slice(1) : size;
        const baseSymbol = row.original.pairId
          .replace("perp/", "")
          .replace("usd", "")
          .toUpperCase();
        return (
          <Cell.Text
            text={
              <>
                <FormattedNumber number={abs} as="span" /> {baseSymbol}
              </>
            }
          />
        );
      },
    },
    {
      header: m["dex.protrade.tradeHistory.tradeValue"](),
      cell: ({ row }) => {
        const { size, price } = row.original;
        if (!size || !price) return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        const absSize = size.startsWith("-") ? size.slice(1) : size;
        const orderValue = Decimal(absSize).times(Decimal(price)).toFixed();
        return (
          <Cell.Text
            text={
              <FormattedNumber number={orderValue} formatOptions={{ currency: "USD" }} as="span" />
            }
          />
        );
      },
    },
    {
      header: m["dex.protrade.history.price"](),
      cell: ({ row }) => {
        const { price } = row.original;
        if (!price) return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        return (
          <Cell.Text
            text={<FormattedNumber number={price} formatOptions={{ currency: "USD" }} as="span" />}
          />
        );
      },
    },
    {
      header: m["dex.protrade.tradeHistory.pnl"](),
      cell: ({ row }) => {
        const { pnl } = row.original;
        if (!pnl || pnl === "0") return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        const isPositive = !pnl.startsWith("-");
        return (
          <Cell.Text
            text={
              <>
                {isPositive ? "+" : ""}
                <FormattedNumber number={pnl} as="span" />
              </>
            }
            className={isPositive ? "text-green-500" : "text-red-500"}
          />
        );
      },
    },
    {
      header: m["dex.protrade.tradeHistory.funding"](),
      cell: ({ row }) => {
        const tradeDate = new Date(row.original.createdAt);
        if (+tradeDate < +V017_CUTOFF) {
          return (
            <Tooltip title={m["dex.protrade.tradeHistory.fundingNotAvailable"]()}>
              <p className="diatype-xs-regular text-ink-tertiary-500 cursor-help underline decoration-dashed underline-offset-[4px] decoration-current">
                N/A
              </p>
            </Tooltip>
          );
        }
        const { funding } = row.original;
        if (!funding || funding === "0")
          return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        const isPositive = !funding.startsWith("-");
        return (
          <Cell.Text
            text={
              <>
                {isPositive ? "+" : ""}
                <FormattedNumber number={funding} as="span" />
              </>
            }
            className={isPositive ? "text-red-500" : "text-green-500"}
          />
        );
      },
    },
    {
      header: m["dex.protrade.tradeHistory.fees"](),
      cell: ({ row }) => {
        const { fee } = row.original;
        if (!fee || fee === "0") return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        return (
          <Cell.Text
            text={<FormattedNumber number={fee} formatOptions={{ currency: "USD" }} as="span" />}
          />
        );
      },
    },
    {
      header: m["dex.protrade.tradeHistory.makerTaker"](),
      cell: ({ row }) => {
        if (row.original.eventType !== "order_filled") {
          return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        }
        const tradeDate = new Date(row.original.createdAt);
        if (tradeDate < V016_CUTOFF) {
          return (
            <Tooltip title={m["dex.protrade.tradeHistory.makerTakerNotAvailable"]()}>
              <p className="diatype-xs-regular text-ink-tertiary-500 cursor-help underline decoration-dashed underline-offset-[4px] decoration-current">
                N/A
              </p>
            </Tooltip>
          );
        }
        const { isMaker } = row.original;
        return (
          <Cell.Text
            text={
              isMaker
                ? m["dex.protrade.tradeHistory.maker"]()
                : m["dex.protrade.tradeHistory.taker"]()
            }
          />
        );
      },
    },
    {
      header: m["dex.protrade.tradeHistory.time"](),
      cell: ({ row }) => <Cell.Time date={row.original.createdAt} dateFormat="MM/dd/yy h:mm a" />,
    },
  ];

  return <TradeHistoryTable data={normalizedData} columns={columns} isLoading={isLoading} />;
};
