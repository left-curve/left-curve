import { Decimal, truncateAddress } from "@left-curve/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { MarketPair } from "@left-curve/foundation/market-pair";
import { getChartResolutionBarTime } from "./chartResolution";
import { normalizePerpsEvent, type NormalizedFields } from "./normalizePerpsEvent";
import { getMakerTakerLabel, getSideLabel } from "./perpsEventLabels";
import { perpsTradeHistoryKeys } from "./perpsTradeHistoryKeys";

import type { PerpsEvent } from "@left-curve/types";
import type { PublicClient } from "@left-curve/sdk";
import type { QueryClient } from "@tanstack/react-query";
import type { Mark, ResolutionString } from "@left-curve/tradingview";

const BUY_COLOR = "#27AE60";
const SELL_COLOR = "#EB5757";
const LABEL_COLOR = "#FFFCF6";
const FILL_MARKERS_RANGE_BUCKET_SECONDS = 24 * 60 * 60;
// Cap a single chart range request; older fills may be omitted for very active accounts.
const FILL_MARKERS_LIMIT = 500;

type BuildFillMarkerParameters = {
  resolution: string;
};

type DecimalValue = ReturnType<typeof Decimal>;

type FetchFillMarkersParameters = {
  client: PublicClient;
  queryClient: QueryClient;
  accountAddress: string;
  pairId: string;
  resolution: ResolutionString;
  from: number;
  to: number;
};

function getFillMarkerRangeBucket(from: number, to: number) {
  return {
    from: Math.floor(from / FILL_MARKERS_RANGE_BUCKET_SECONDS) * FILL_MARKERS_RANGE_BUCKET_SECONDS,
    to:
      (Math.floor(to / FILL_MARKERS_RANGE_BUCKET_SECONDS) + 1) * FILL_MARKERS_RANGE_BUCKET_SECONDS,
  };
}

function buildMarkerText(
  event: PerpsEvent,
  fields: NormalizedFields,
  size: DecimalValue,
  price: DecimalValue,
): string[] {
  const baseSymbol = MarketPair.fromPairId(event.pairId).base.symbol;
  const sideLabel = getSideLabel(size.lt(0));
  const text = [`${sideLabel} ${size.abs().toFixed()} ${baseSymbol} at $${price.toFixed()}`];

  if (fields.fee !== undefined) {
    text.push(`${m["dex.protrade.tradeHistory.fees"]()}: $${fields.fee}`);
  }

  if (fields.pnl !== undefined) {
    text.push(`${m["dex.protrade.tradeHistory.pnl"]()}: ${fields.pnl}`);
  }

  if (fields.funding !== undefined) {
    text.push(`${m["dex.protrade.tradeHistory.funding"]()}: ${fields.funding}`);
  }

  if (fields.isMaker !== undefined) {
    text.push(getMakerTakerLabel(fields.isMaker));
  }

  text.push(`${m["dex.protrade.tradeHistory.time"]()}: ${event.createdAt}`);
  if (event.txHash) text.push(`Tx: ${truncateAddress(event.txHash, 6)}`);

  return text;
}

export function buildFillMarker(
  event: PerpsEvent,
  parameters: BuildFillMarkerParameters,
): Mark | null {
  if (event.eventType !== "order_filled") return null;

  const fields = normalizePerpsEvent(event);
  const fillTimeMs = Date.parse(event.createdAt);
  if (!Number.isFinite(fillTimeMs)) return null;

  if (!fields.size || !fields.price) return null;

  const size = Decimal(fields.size);
  const price = Decimal(fields.price);
  if (size.isZero() || !Number.isFinite(price.toNumber())) return null;

  const time = getChartResolutionBarTime(fillTimeMs, parameters.resolution);
  if (time === undefined) return null;

  const isShort = size.lt(0);
  const color = isShort ? SELL_COLOR : BUY_COLOR;

  return {
    id: `${event.txHash}:${event.idx}`,
    time,
    color: {
      border: color,
      background: color,
    },
    text: buildMarkerText(event, fields, size, price).join("\n"),
    label: isShort ? "S" : "B",
    labelFontColor: LABEL_COLOR,
    minSize: 16,
  };
}

export async function fetchFillMarkers({
  client,
  queryClient,
  accountAddress,
  pairId,
  resolution,
  from,
  to,
}: FetchFillMarkersParameters): Promise<Mark[]> {
  const rangeBucket = getFillMarkerRangeBucket(from, to);
  const earlierThan = new Date(rangeBucket.to * 1000);
  const laterThan = new Date(rangeBucket.from * 1000);

  const { nodes } = await queryClient.fetchQuery({
    queryKey: perpsTradeHistoryKeys.fillMarkers(
      accountAddress,
      pairId,
      rangeBucket.from,
      rangeBucket.to,
    ),
    staleTime: 10_000,
    queryFn: () =>
      client.queryPerpsEvents({
        userAddr: accountAddress,
        pairId,
        eventType: "order_filled",
        sortBy: "BLOCK_HEIGHT_DESC",
        first: FILL_MARKERS_LIMIT,
        earlierThan: earlierThan.toJSON(),
        laterThan: laterThan.toJSON(),
      }),
  });

  return nodes
    .flatMap((event) => {
      const marker = buildFillMarker(event, { resolution });
      return marker ? [marker] : [];
    })
    .sort((a, b) => a.time - b.time);
}
