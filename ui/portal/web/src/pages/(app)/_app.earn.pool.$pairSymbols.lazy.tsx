import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";
import { PoolLiquidity } from "~/components/earn/PoolLiquidity";

import { motion } from "framer-motion";

export const Route = createLazyFileRoute("/(app)/_app/earn/pool/$pairSymbols")({
  component: PoolApplet,
});

function PoolApplet() {
  const { pair, config } = Route.useRouteContext();
  const { action } = Route.useSearch();

  const navigate = useNavigate();

  const onChangeAction = (action: "deposit" | "withdraw") => {
    const { coins } = config;
    navigate({
      to: "/earn/pool/$pairSymbols",
      params: {
        pairSymbols: `${coins.byDenom[pair.baseDenom].symbol}-${coins.byDenom[pair.quoteDenom].symbol}`,
      },
      replace: true,
      search: { action },
    });
  };

  return (
    <PoolLiquidity pair={pair} action={action} onChangeAction={onChangeAction}>
      <PoolLiquidity.Header />
      <motion.div layout="position" className="flex w-full gap-8 lg:gap-6 flex-col lg:flex-row">
        <div className="flex flex-col flex-1 min-w-0 gap-4">
          <PoolLiquidity.HeaderTabs />
          <PoolLiquidity.Deposit />
          <PoolLiquidity.Withdraw />
        </div>
        <PoolLiquidity.UserLiquidity />
      </motion.div>
    </PoolLiquidity>
  );
}
