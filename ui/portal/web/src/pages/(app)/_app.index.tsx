import { useAccount, useBalances } from "@left-curve/store-react";
import { createFileRoute } from "@tanstack/react-router";
import { useApp } from "~/hooks/useApp";

import { IconButton, IconChevronDown, PoolTable, StrategyCard } from "@left-curve/applets-kit";
import { useRef, useState } from "react";
import { AssetsPreview } from "~/components/AssetsPreview";
import { ButtonLink } from "~/components/ButtonLink";
import { FavAppletSection } from "~/components/FavAppletSection";
import { DotsIndicator, SwippeableAccountCard } from "~/components/SwippeableAccountCard";
import { m } from "~/paraglide/messages";

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
  const [cardMobileVisible, setCardMobileVisible] = useState(0);

  const { data: balances = {} } = useBalances({ address: account?.address });
  const topYieldsRef = useRef<HTMLDivElement>(null);

  const scrollToSection = () => {
    topYieldsRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  return (
    <div className="w-full lg:max-w-[76rem] mx-auto flex flex-col gap-8 p-4 pb-32">
      <div className="w-full flex flex-col gap-8 min-h-[100dvh] lg:min-h-fit relative">
        <div className="rounded-3xl bg-rice-50 shadow-card-shadow flex flex-col lg:flex-row gap-4 w-full p-4 items-center lg:items-start">
          <SwippeableAccountCard
            cardVisible={cardMobileVisible}
            setCardVisible={setCardMobileVisible}
          />

          <div className="w-full flex flex-col lg:gap-4 items-center">
            <div className="hidden lg:flex w-full">
              <AssetsPreview
                balances={balances}
                showAllAssets={isConnected ? () => setSidebarVisibility(true) : undefined}
              />
            </div>

            {isConnected ? (
              <div className="lg:self-end flex gap-4 items-center justify-center w-full lg:max-w-[256px]">
                <ButtonLink
                  fullWidth
                  size="md"
                  to="/send-and-receive"
                  search={{ action: "receive" }}
                >
                  {m["common.funds"]()}
                </ButtonLink>
                <ButtonLink
                  fullWidth
                  variant="secondary"
                  size="md"
                  to="/send-and-receive"
                  search={{ action: "send" }}
                >
                  {m["common.send"]()}
                </ButtonLink>
              </div>
            ) : null}
          </div>
        </div>

        <DotsIndicator cardVisible={cardMobileVisible} setCardVisible={setCardMobileVisible} />

        <FavAppletSection />
        <IconButton
          onClick={scrollToSection}
          variant="link"
          className="absolute left-1/2 -translate-x-1/2 bottom-24 z-30 lg:hidden"
        >
          <IconChevronDown />
        </IconButton>
      </div>

      {/*   <div
        ref={topYieldsRef}
        className="bg-rice-25 shadow-card-shadow flex flex-col rounded-3xl w-full pt-4"
      >
        <p className="h3-heavy font-extrabold px-4 py-3">Top Yields</p>

        <div className="flex gap-6 w-full overflow-y-scroll p-4 scrollbar-none">
          {Array.from([1, 2, 3]).map(() => (
            <StrategyCard key={crypto.randomUUID()} />
          ))}
        </div>
      </div> */}

      {/*  <PoolTable data={mockDataTable} /> */}
    </div>
  );
}
