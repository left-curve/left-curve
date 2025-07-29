import { createFileRoute, redirect } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "~/paraglide/messages";

import { useConfig } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";

import { ProTrade } from "~/components/dex/ProTrade";

import type { PairId } from "@left-curve/dango/types";

export const Route = createFileRoute("/(app)/_app/trade/$pairSymbols")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.0.title"]()}` }],
  }),
  beforeLoad: async ({ context, params }) => {
    const { client, config } = context;
    const { coins } = config;
    const { pairSymbols } = params;
    const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
    const baseDenom = coins.bySymbol[baseSymbol]?.denom;
    const quoteDenom = coins.bySymbol[quoteSymbol]?.denom;

    const pair = await client?.getPair({ baseDenom, quoteDenom }).catch(() => null);
    if (!pair)
      throw redirect({
        to: "/trade/$pairSymbols",
        params: { pairSymbols: "BTC-USDC" },
      });
  },
  validateSearch: z.object({
    order_type: z.enum(["limit", "market"]).default("market"),
    action: z.enum(["buy", "sell"]).default("buy"),
  }),
  component: ProTradeApplet,
  wrapInSuspense: true,
});

function ProTradeApplet() {
  const navigate = useNavigate();
  const { coins } = useConfig();
  const { pairSymbols } = Route.useParams();
  const { action = "buy", order_type = "market" } = Route.useSearch();

  const onChangePairId = ({ baseDenom, quoteDenom }: PairId) => {
    const baseSymbol = coins.byDenom[baseDenom]?.symbol;
    const quoteSymbol = coins.byDenom[quoteDenom]?.symbol;

    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols: `${baseSymbol}-${quoteSymbol}` },
      replace: false,
    });
  };

  const onChangeAction = (action: "buy" | "sell") => {
    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols },
      replace: false,
      search: { order_type, action },
    });
  };

  const onChangeOrderType = (order_type: "limit" | "market") => {
    navigate({
      to: "/trade/$pairSymbols",
      params: { pairSymbols },
      replace: false,
      search: { order_type, action },
    });
  };

  const [baseSymbol, quoteSymbol] = pairSymbols.split("-");

  const pairId = {
    baseDenom: coins.bySymbol[baseSymbol]?.denom,
    quoteDenom: coins.bySymbol[quoteSymbol]?.denom,
  };

  return (
    <div className="flex w-full min-h-screen lg:min-h-[calc(100vh-76px)] relative overflow-visible">
      <ProTrade
        pairId={pairId}
        onChangePairId={onChangePairId}
        action={action}
        onChangeAction={onChangeAction}
        orderType={order_type}
        onChangeOrderType={onChangeOrderType}
      >
        <div className="flex flex-col flex-1">
          <div className="flex flex-col xl:flex-row flex-1">
            <div className="flex flex-col flex-1">
              <ProTrade.Header />
              <ProTrade.Chart />
            </div>
            <ProTrade.OrderBook />
          </div>
          <ProTrade.Orders />
        </div>
        <div className="hidden lg:flex pt-4 w-full lg:w-[331px] xl:[width:clamp(279px,20vw,330px)] lg:bg-surface-secondary-rice shadow-account-card z-20 max-h-[calc(100vh-76px)] md:sticky top-[76px]">
          <ProTrade.TradeMenu />
        </div>
      </ProTrade>
    </div>
  );
}
