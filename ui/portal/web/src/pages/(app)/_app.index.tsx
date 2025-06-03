import { createFileRoute } from "@tanstack/react-router";

import { IconButton, IconChevronDown, IconInfo, Tooltip } from "@left-curve/applets-kit";
import { useAccount } from "@left-curve/store";
import { useRef, useState } from "react";
import { AppletsSection } from "~/components/overview/AppletsSection";
import { DotsIndicator } from "~/components/overview/SwippeableAccountCard";
import { WelcomeSection } from "~/components/overview/WelcomeSection";

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
  const { isConnected } = useAccount();
  const [cardMobileVisible, setCardMobileVisible] = useState(0);

  const topYieldsRef = useRef<HTMLDivElement>(null);

  const scrollToSection = () => {
    topYieldsRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  return (
    <div className="w-full lg:max-w-[76rem] mx-auto flex flex-col gap-6 p-4 pt-6 mb-16">
      <div className="w-full flex flex-col gap-6 min-h-[100dvh] lg:min-h-fit relative">
        <WelcomeSection
          cardMobileVisible={cardMobileVisible}
          setCardMobileVisible={setCardMobileVisible}
        />

        {isConnected && (
          <DotsIndicator cardVisible={cardMobileVisible} setCardVisible={setCardMobileVisible} />
        )}

        <AppletsSection />
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
        className="bg-rice-25 shadow-account-card flex flex-col rounded-xl w-full pt-4"
      >
        <p className="h3-heavy font-extrabold px-4 py-3">Top Yields</p>

        <div className="flex gap-6 w-full overflow-y-scroll p-4 scrollbar-none">
          {Array.from([1, 2, 3]).map(() => (
            <StrategyCard key={crypto.randomUUID()} />
          ))}
        </div>
      </div> */}
      {/*  <PoolTable data={mockDataTable} /> */}
      <Tooltip content="This is a tooltip example">
        <IconInfo className="w-6 h-6" />
      </Tooltip>
      <Tooltip content="This is a tooltip example">
        <div className="flex items-center gap-2">
          <p>Leverage</p>
          <IconInfo className="w-6 h-6" />
        </div>
      </Tooltip>
      <Tooltip placement="auto" content="This is a tooltip example">
        <div className="flex items-center gap-2">
          <p>Leverage</p>
          <IconInfo className="w-6 h-6" />
        </div>
      </Tooltip>
      <Tooltip placement="auto" content="This is a tooltip example" delay={2000}>
        <div className="flex items-center gap-2">
          <p>With delay 2s</p>
        </div>
      </Tooltip>
      <Tooltip placement="auto" content="This is a tooltip example" closeDelay={2000}>
        <div className="flex items-center gap-2">
          <p>With close delay 2s</p>
        </div>
      </Tooltip>
      <Tooltip placement="bottom" content="This is a tooltip example">
        <div className="flex items-center gap-2">
          <p>Leverage</p>
          <IconInfo className="w-6 h-6" />
        </div>
      </Tooltip>
      <Tooltip placement="left" content="This is a tooltip example">
        <div className="flex items-center gap-2">
          <p>Leverage</p>
          <IconInfo className="w-6 h-6" />
        </div>
      </Tooltip>
      <Tooltip placement="right" content="This is a tooltip example">
        <div className="flex items-center gap-2">
          <p>Leverage</p>
          <IconInfo className="w-6 h-6" />
        </div>
      </Tooltip>
    </div>
  );
}
