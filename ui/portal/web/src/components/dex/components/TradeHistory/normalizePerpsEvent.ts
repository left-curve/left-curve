import type {
  DeleveragedData,
  LiquidatedData,
  OrderFilledData,
  PerpsEvent,
} from "@left-curve/types";

export type NormalizedFields = {
  size: string | undefined;
  price: string | undefined;
  pnl: string | undefined;
  fee: string | undefined;
  funding: string | undefined;
  isMaker: boolean | undefined;
};

const orUndefined = <T>(value: T | null | undefined): T | undefined =>
  value === null ? undefined : value;

export function normalizePerpsEvent(event: PerpsEvent): NormalizedFields {
  switch (event.eventType) {
    case "order_filled": {
      const d = event.data as OrderFilledData;
      return {
        size: d.fill_size,
        price: d.fill_price,
        pnl: d.realized_pnl,
        fee: d.fee,
        funding: orUndefined(d.realized_funding),
        isMaker: orUndefined(d.is_maker),
      };
    }
    case "liquidated": {
      const d = event.data as LiquidatedData;
      return {
        size: d.adl_size,
        price: orUndefined(d.adl_price),
        pnl: d.adl_realized_pnl,
        fee: undefined,
        funding: orUndefined(d.adl_realized_funding),
        isMaker: undefined,
      };
    }
    case "deleveraged": {
      const d = event.data as DeleveragedData;
      return {
        size: d.closing_size,
        price: d.fill_price,
        pnl: d.realized_pnl,
        fee: undefined,
        funding: orUndefined(d.realized_funding),
        isMaker: undefined,
      };
    }
    default:
      return {
        size: undefined,
        price: undefined,
        pnl: undefined,
        fee: undefined,
        funding: undefined,
        isMaker: undefined,
      };
  }
}
