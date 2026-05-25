import { Cell, FormattedNumber, Spinner, Tooltip, twMerge } from "@left-curve/applets-kit";
import { Decimal } from "@left-curve/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useNavigate } from "@tanstack/react-router";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect, useMemo, useRef } from "react";

import { EmptyPlaceholder } from "../../../foundation/EmptyPlaceholder";
import { useTradeHistoryFilter } from "./tradeHistoryFilterContext";

import type {
  DeleveragedData,
  LiquidatedData,
  OrderFilledData,
  PerpsEvent,
} from "@left-curve/types";

const eventTypeLabels: Record<string, string> = {
  order_filled: "Trade",
  liquidated: "Liquidation",
  deleveraged: "ADL",
};

const V016_CUTOFF = new Date("2026-04-22T12:00:00Z");
const V017_CUTOFF = new Date("2026-04-30T12:00:00Z");

// Roughly the rendered row height — virtualizer adapts via measureElement,
// but a sensible estimate keeps the initial scroll math accurate.
const ROW_HEIGHT = 32;
// Prefetch the next page when the user is within this many rows of the end.
const FETCH_NEXT_THRESHOLD = 10;

type NormalizedFields = {
  size: string | null | undefined;
  price: string | null | undefined;
  pnl: string | null | undefined;
  fee: string | null | undefined;
  funding: string | null | undefined;
  isMaker: boolean | null | undefined;
};

function normalizePerpsEvent(event: PerpsEvent): NormalizedFields {
  switch (event.eventType) {
    case "order_filled": {
      const d = event.data as OrderFilledData;
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
      const d = event.data as LiquidatedData;
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
      const d = event.data as DeleveragedData;
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

type ColumnDef = {
  key: string;
  header: string;
  // Width string passed to grid-template-columns. `1fr` for the stretchy
  // first column, fixed/min widths for the rest so columns stay aligned.
  width: string;
  render: (event: PerpsEvent, fields: NormalizedFields) => React.ReactNode;
};

const columns: ColumnDef[] = [
  {
    key: "pair",
    header: m["dex.protrade.tradeHistory.pair"](),
    width: "minmax(80px, 1fr)",
    render: (event) => {
      const pair = event.pairId.replace("perp/", "").replace("usd", "/USD").toUpperCase();
      return <Cell.Text text={pair} className="diatype-xs-medium" />;
    },
  },
  {
    key: "type",
    header: m["dex.protrade.history.type"](),
    width: "minmax(80px, 1fr)",
    render: (event) => <Cell.Text text={eventTypeLabels[event.eventType] ?? event.eventType} />,
  },
  {
    key: "direction",
    header: m["dex.protrade.tradeHistory.direction"](),
    width: "minmax(80px, 1fr)",
    render: (_, { size }) => {
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
    key: "size",
    header: "Size",
    width: "minmax(120px, 1.4fr)",
    render: (event, { size }) => {
      if (!size) return <Cell.Text text="-" className="text-ink-tertiary-500" />;
      const abs = size.startsWith("-") ? size.slice(1) : size;
      const baseSymbol = event.pairId.replace("perp/", "").replace("usd", "").toUpperCase();
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
    key: "tradeValue",
    header: m["dex.protrade.tradeHistory.tradeValue"](),
    width: "minmax(110px, 1.2fr)",
    render: (_, { size, price }) => {
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
    key: "price",
    header: m["dex.protrade.history.price"](),
    width: "minmax(90px, 1fr)",
    render: (_, { price }) => {
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
    key: "pnl",
    header: m["dex.protrade.tradeHistory.pnl"](),
    width: "minmax(90px, 1fr)",
    render: (_, { pnl }) => {
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
    key: "funding",
    header: m["dex.protrade.tradeHistory.funding"](),
    width: "minmax(80px, 1fr)",
    render: (event, { funding }) => {
      const tradeDate = new Date(event.createdAt);
      if (+tradeDate < +V017_CUTOFF) {
        return (
          <Tooltip title={m["dex.protrade.tradeHistory.fundingNotAvailable"]()}>
            <p className="diatype-xs-regular text-ink-tertiary-500 cursor-help underline decoration-dashed underline-offset-[4px] decoration-current">
              N/A
            </p>
          </Tooltip>
        );
      }
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
    key: "fees",
    header: m["dex.protrade.tradeHistory.fees"](),
    width: "minmax(90px, 1fr)",
    render: (_, { fee }) => {
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
    key: "makerTaker",
    header: m["dex.protrade.tradeHistory.makerTaker"](),
    width: "minmax(80px, 1fr)",
    render: (event, { isMaker }) => {
      if (event.eventType !== "order_filled") {
        return <Cell.Text text="-" className="text-ink-tertiary-500" />;
      }
      const tradeDate = new Date(event.createdAt);
      if (tradeDate < V016_CUTOFF) {
        return (
          <Tooltip title={m["dex.protrade.tradeHistory.makerTakerNotAvailable"]()}>
            <p className="diatype-xs-regular text-ink-tertiary-500 cursor-help underline decoration-dashed underline-offset-[4px] decoration-current">
              N/A
            </p>
          </Tooltip>
        );
      }
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
    key: "time",
    header: m["dex.protrade.tradeHistory.time"](),
    width: "minmax(130px, 1.1fr)",
    render: (event) => <Cell.Time date={event.createdAt} dateFormat="MM/dd/yy h:mm a" />,
  },
];

const gridTemplate = columns.map((c) => c.width).join(" ");

export const PerpsTradeHistory: React.FC = () => {
  const navigate = useNavigate();
  const { nodes, isLoading, isFetchingNextPage, hasNextPage, fetchNextPage } =
    useTradeHistoryFilter();

  const scrollRef = useRef<HTMLDivElement | null>(null);

  const normalizedNodes = useMemo(
    () => nodes.map((event) => ({ event, fields: normalizePerpsEvent(event) })),
    [nodes],
  );

  const virtualizer = useVirtualizer({
    count: normalizedNodes.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  });

  const virtualItems = virtualizer.getVirtualItems();

  // Auto-load the next page once the bottom of the viewport reaches the
  // threshold. The context guard avoids overlapping fetches.
  useEffect(() => {
    const last = virtualItems[virtualItems.length - 1];
    if (!last) return;
    if (last.index >= normalizedNodes.length - FETCH_NEXT_THRESHOLD) {
      fetchNextPage();
    }
  }, [virtualItems, normalizedNodes.length, fetchNextPage]);

  const showEmpty = !isLoading && normalizedNodes.length === 0;

  return (
    <div ref={scrollRef} className="relative w-full overflow-y-auto overflow-x-auto max-h-[60vh]">
      <div
        className="grid sticky top-0 z-10 bg-surface-primary-rice diatype-xs-medium text-ink-tertiary-500 px-1 py-2 border-b border-outline-secondary-gray"
        style={{ gridTemplateColumns: gridTemplate, minWidth: "fit-content" }}
      >
        {columns.map((col) => (
          <div key={col.key} className="px-2">
            {col.header}
          </div>
        ))}
      </div>

      {showEmpty ? (
        <EmptyPlaceholder
          component={m["dex.protrade.history.noOpenOrders"]()}
          className="h-[3.5rem]"
        />
      ) : (
        <div
          style={{
            height: `${virtualizer.getTotalSize()}px`,
            position: "relative",
            minWidth: "fit-content",
          }}
        >
          {virtualItems.map((virtualRow) => {
            const item = normalizedNodes[virtualRow.index];
            if (!item) return null;
            return (
              <button
                key={virtualRow.key}
                type="button"
                onClick={() =>
                  navigate({
                    to: "/block/$block",
                    params: { block: item.event.blockHeight.toString() },
                  })
                }
                className={twMerge(
                  "grid items-center w-full text-left px-1 diatype-xs-regular cursor-pointer transition-colors hover:bg-surface-secondary-rice",
                )}
                style={{
                  gridTemplateColumns: gridTemplate,
                  position: "absolute",
                  top: 0,
                  left: 0,
                  transform: `translateY(${virtualRow.start}px)`,
                  height: `${virtualRow.size}px`,
                  minWidth: "fit-content",
                }}
              >
                {columns.map((col) => (
                  <div key={col.key} className="px-2 py-1">
                    {col.render(item.event, item.fields)}
                  </div>
                ))}
              </button>
            );
          })}
        </div>
      )}

      {isFetchingNextPage ? (
        <div className="flex items-center justify-center py-3">
          <Spinner color="pink" size="sm" />
        </div>
      ) : null}

      {isLoading && normalizedNodes.length === 0 ? (
        <div className="flex items-center justify-center py-6">
          <Spinner color="pink" size="md" />
        </div>
      ) : null}

      {!hasNextPage && !isLoading && normalizedNodes.length > 0 ? (
        <div className="text-center text-ink-tertiary-500 diatype-xs-regular py-2">
          {/* End-of-list marker keeps the layout from feeling truncated. */}—
        </div>
      ) : null}
    </div>
  );
};
