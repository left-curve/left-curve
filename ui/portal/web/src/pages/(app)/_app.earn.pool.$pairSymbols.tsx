import { Button, Tabs, twMerge } from "@left-curve/applets-kit";
import { createFileRoute, redirect } from "@tanstack/react-router";
import { useState } from "react";
import { Earn } from "~/components/earn/Earn";
import { coinsBySymbol } from "~/store";
import { motion } from "framer-motion";
import { PoolLiquidity } from "~/components/earn/PoolLiquidity";

export const Route = createFileRoute("/(app)/_app/earn/pool/$pairSymbols")({
  component: RouteComponent,
  beforeLoad: async ({ context, params }) => {
    const { client } = context;
    const { pairSymbols } = params;
    const [baseSymbol, quoteSymbol] = pairSymbols.split("-");
    const baseDenom = coinsBySymbol[baseSymbol]?.denom;
    const quoteDenom = coinsBySymbol[quoteSymbol]?.denom;

    const pair = await client?.getPair({ baseDenom, quoteDenom }).catch(() => null);
    if (!pair)
      throw redirect({ to: "/earn/pool/$pairSymbols", params: { pairSymbols: "BTC-USDC" } });
  },
  wrapInSuspense: true,
});

function RouteComponent() {
  const { pairSymbols } = Route.useParams();

  const [baseSymbol, quoteSymbol] = pairSymbols.split("-");

  const pair = {
    base: coinsBySymbol[baseSymbol],
    quote: coinsBySymbol[quoteSymbol],
  };

  const userHavePosition = false;

  return (
    <PoolLiquidity pairId={{ baseDenom: pair.base.denom, quoteDenom: pair.quote.denom }}>
      <PoolLiquidity.Header />
      <div className="flex w-full gap-8 lg:gap-6 flex-col lg:flex-row">
        <PoolLiquidity.DepositWithdraw />
        <PoolLiquidity.UserLiquidity />
      </div>
    </PoolLiquidity>
  );
}
