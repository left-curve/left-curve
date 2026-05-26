import { Cell, FormattedNumber, Tooltip } from "@left-curve/applets-kit";
import { useAccount, usePublicClient, useQueryWithPagination } from "@left-curve/store";
import { Decimal } from "@left-curve/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import { TradeHistoryTable } from "./TradeHistoryTable";
import { normalizePerpsEvent } from "./normalizePerpsEvent";
import { getMakerTakerLabel, getPerpsEventLabel, getSideLabel } from "./perpsEventLabels";

import type { TableColumn } from "@left-curve/applets-kit";
import type { PerpsEvent } from "@left-curve/types";

const V016_CUTOFF = new Date("2026-04-22T12:00:00Z");
const V017_CUTOFF = new Date("2026-04-30T12:00:00Z");

type NormalizedPerpsEvent = PerpsEvent & ReturnType<typeof normalizePerpsEvent>;

export function LegacyPerpsTradeHistory() {
  const { account } = useAccount();
  const publicClient = usePublicClient();
  const { data, isLoading } = useQueryWithPagination({
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
          ...normalizePerpsEvent(event),
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
      cell: ({ row }) => <Cell.Text text={getPerpsEventLabel(row.original.eventType)} />,
    },
    {
      header: m["dex.protrade.tradeHistory.direction"](),
      cell: ({ row }) => {
        const { size } = row.original;
        if (!size) return <Cell.Text text="-" className="text-ink-tertiary-500" />;
        const isShort = size.startsWith("-");
        return (
          <Cell.Text
            text={getSideLabel(isShort)}
            className={isShort ? "text-red-500" : "text-green-500"}
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
                <FormattedNumber number={abs} formatOptions={{ maxFractionDigits: 6 }} as="span" />{" "}
                {baseSymbol}
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
              <FormattedNumber
                number={orderValue}
                formatOptions={{ currency: "USD", maxFractionDigits: 6 }}
                as="span"
              />
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
            text={
              <FormattedNumber
                number={price}
                formatOptions={{ currency: "USD", maxFractionDigits: 6 }}
                as="span"
              />
            }
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
                <FormattedNumber number={pnl} formatOptions={{ maxFractionDigits: 6 }} as="span" />
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
                <FormattedNumber
                  number={funding}
                  formatOptions={{ maxFractionDigits: 6 }}
                  as="span"
                />
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
            text={
              <FormattedNumber
                number={fee}
                formatOptions={{ currency: "USD", maxFractionDigits: 6 }}
                as="span"
              />
            }
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
        return <Cell.Text text={getMakerTakerLabel(!!row.original.isMaker)} />;
      },
    },
    {
      header: m["dex.protrade.tradeHistory.time"](),
      cell: ({ row }) => <Cell.Time date={row.original.createdAt} dateFormat="MM/dd/yy h:mm a" />,
    },
  ];

  return <TradeHistoryTable data={normalizedData} columns={columns} isLoading={isLoading} />;
}
