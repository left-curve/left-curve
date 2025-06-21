import { createLazyFileRoute, useNavigate } from "@tanstack/react-router";
import { PoolLiquidity } from "~/components/earn/PoolLiquidity";

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
      params: { pairSymbols: `${coins[pair.baseDenom].symbol}-${coins[pair.quoteDenom].symbol}` },
      replace: false,
      search: { action },
    });
  };

  return (
    <PoolLiquidity pair={pair} action={action} onChangeAction={onChangeAction}>
      <PoolLiquidity.Header />
      <div className="flex w-full gap-8 lg:gap-6 flex-col lg:flex-row">
        <div className="flex flex-col w-full gap-4">
          <PoolLiquidity.HeaderTabs />
          <PoolLiquidity.Deposit />
          <PoolLiquidity.Withdraw />
        </div>
        <PoolLiquidity.UserLiquidity />
      </div>
    </PoolLiquidity>
  );
}
