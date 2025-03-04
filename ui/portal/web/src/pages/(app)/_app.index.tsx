import { Link, createFileRoute } from "@tanstack/react-router";

import {
  AccountCard,
  AssetsPreview,
  Button,
  IconAddCross,
  PoolTable,
  StrategyCard,
} from "@left-curve/applets-kit";
import { useAccount, useBalances, usePrices } from "@left-curve/store-react";
import { ButtonLink } from "~/components/ButtonLink";
import { useApp } from "~/hooks/useApp";

const mockDataTable = [
  {
    vault: "ETH-USD",
    type: "Lending",
    apr: "17.72%",
    liquidity: "15.63%",
    tvl: "15.63%",
    risk: "Low",
  },
  {
    vault: "ETH-USD",
    type: "Lending",
    apr: "17.72%",
    liquidity: "15.63%",
    tvl: "15.63%",
    risk: "Low",
  },
  {
    vault: "ETH-USD",
    type: "Lending",
    apr: "17.72%",
    liquidity: "15.63%",
    tvl: "15.63%",
    risk: "Low",
  },
  {
    vault: "ETH-USD",
    type: "Lending",
    apr: "17.72%",
    liquidity: "15.63%",
    tvl: "15.63%",
    risk: "Low",
  },
  {
    vault: "ETH-USD",
    type: "Lending",
    apr: "17.72%",
    liquidity: "15.63%",
    tvl: "15.63%",
    risk: "Low",
  },
];

export const Route = createFileRoute("/(app)/_app/")({
  component: OverviewComponent,
});

function OverviewComponent() {
  const { account, isConnected } = useAccount();
  const { setSidebarVisibility } = useApp();

  const { data: balances = {} } = useBalances({ address: account?.address });
  const { calculateBalance } = usePrices();
  const totalBalance = calculateBalance(balances, { format: true });

  return (
    <div className="w-full  md:max-w-[76rem] mx-auto flex flex-col gap-8 p-4">
      <div className="rounded-3xl bg-rice-50 shadow-card-shadow flex flex-col md:flex-row gap-4 w-full p-4 items-center md:items-start">
        {isConnected ? (
          <AccountCard account={account!} balance={totalBalance} />
        ) : (
          <AccountCard
            account={{
              address: "0x000000",
              index: 0,
              type: "spot",
              username: "username",
              params: {
                spot: { owner: "username" },
              },
            }}
            balance=""
          />
        )}
        <div className="w-full flex flex-col gap-4 items-center">
          <AssetsPreview
            balances={balances}
            showAllAssets={isConnected ? () => setSidebarVisibility(true) : undefined}
          />

          {isConnected ? (
            <div className="md:self-end flex gap-4 items-center justify-center w-full md:max-w-[256px]">
              <ButtonLink fullWidth size="md" to="/send-and-receive" search={{ action: "receive" }}>
                Fund
              </ButtonLink>
              <ButtonLink
                fullWidth
                variant="secondary"
                size="md"
                to="/send-and-receive"
                search={{ action: "send" }}
              >
                Send
              </ButtonLink>
            </div>
          ) : null}
        </div>
      </div>
      {/* second component */}
      <div className="flex gap-4 md:gap-8 items-start flex-wrap md:justify-start w-full">
        {/* applets items */}
        <div className="flex flex-col items-center gap-2">
          <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-green-bean-50 rounded-md p-[10px]">
            <img src="/images/applets/swap.svg" alt="" className="w-full h-full" />
          </div>
          <p className="text-sm font-bold">Swap</p>
        </div>
        <div className="flex flex-col items-center gap-2">
          <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-red-bean-50 rounded-md p-[10px]">
            <img src="/images/applets/earn.svg" alt="" className="w-full h-full" />
          </div>
          <p className="text-sm font-bold">Earn</p>
        </div>
        <div className="flex flex-col items-center gap-2">
          <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow bg-rice-50 rounded-md p-[10px]">
            <img src="/images/applets/multisig.svg" alt="" className="w-full h-full" />
          </div>
          <p className="text-sm font-bold">Multisig</p>
        </div>
        {/* add applets item */}
        <div className="h-16 w-16 md:h-20 md:w-20 shadow-card-shadow border-[1.43px] border-rice-100 text-rice-100 rounded-md p-[10px] flex items-center justify-center">
          <IconAddCross />
        </div>
      </div>

      {/* <div className="bg-rice-25 shadow-card-shadow flex flex-col rounded-3xl w-full">
        <p className="h3-heavy font-extrabold px-4 py-3">Top Yields</p>

        <div className="flex gap-6 w-full overflow-y-scroll p-4 scrollbar-none">
          {Array.from([1, 2, 3]).map(() => (
            <StrategyCard key={crypto.randomUUID()} />
          ))}
        </div>
      </div>

      <PoolTable data={mockDataTable} /> */}
    </div>
  );
}
