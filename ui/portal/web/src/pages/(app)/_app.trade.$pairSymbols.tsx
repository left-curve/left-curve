import { createFileRoute, redirect } from "@tanstack/react-router";

import { z } from "zod";
import { m } from "~/paraglide/messages";
import { coinsBySymbol } from "~/store";

import { useConfig } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";

import { ProTrade } from "~/components/dex/ProTrade";

import type { PairId } from "@left-curve/dango/types";
import { ChartIQ } from "~/components/foundation/ChartIQ";

export const Route = createFileRoute("/(app)/_app/trade/$pairSymbols")({
  head: () => ({
    meta: [{ title: `Dango | ${m["applets.0.title"]()}` }],
  }),
  beforeLoad: async ({ context, params }) => {
    const { client } = context;
    const { pairSymbols } = params;
    const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
    const baseDenom = coinsBySymbol[baseSymbol]?.denom;
    const quoteDenom = coinsBySymbol[quoteSymbol]?.denom;

    const pair = await client?.getPair({ baseDenom, quoteDenom }).catch(() => null);
    if (!pair)
      throw redirect({
        to: "/trade/$pairSymbols",
        params: { pairSymbols: "BTC-USDC" },
      });
  },
  validateSearch: z.object({
    action: z.enum(["buy", "sell"]).default("buy"),
  }),
  component: ProTradeApplet,
});

function ProTradeApplet() {
  const navigate = useNavigate();
  const { coins } = useConfig();
  const { pairSymbols } = Route.useParams();
  const { action = "buy" } = Route.useSearch();

  const onChangePairId = ({ baseDenom, quoteDenom }: PairId) => {
    const baseSymbol = coins[baseDenom]?.symbol;
    const quoteSymbol = coins[quoteDenom]?.symbol;

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
      search: { action },
    });
  };

  const [baseSymbol, quoteSymbol] = pairSymbols.split("-");

  const pairId = {
    baseDenom: coinsBySymbol[baseSymbol]?.denom,
    quoteDenom: coinsBySymbol[quoteSymbol]?.denom,
  };

  return (
    <div className="flex w-full min-h-screen lg:min-h-[calc(100vh-76px)] relative overflow-visible">
      <ProTrade
        pairId={pairId}
        onChangePairId={onChangePairId}
        action={action}
        onChangeAction={onChangeAction}
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
        <div className="hidden lg:flex pt-4 w-full lg:w-[331px] xl:[width:clamp(279px,20vw,330px)] lg:bg-bg-secondary-rice shadow-card-shadow z-20 max-h-[calc(100vh-76px)] md:sticky top-[76px]">
          <ProTrade.TradeMenu />
        </div>
      </ProTrade>
    </div>
  );
}
