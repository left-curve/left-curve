import type {
  DeleveragedData,
  LiquidatedData,
  OrderFilledData,
  PerpsEvent,
} from "@left-curve/types";

export type NormalizedFields = {
  size: string | null | undefined;
  price: string | null | undefined;
  pnl: string | null | undefined;
  fee: string | null | undefined;
  funding: string | null | undefined;
  isMaker: boolean | null | undefined;
};

export function normalizePerpsEvent(event: PerpsEvent): NormalizedFields {
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
