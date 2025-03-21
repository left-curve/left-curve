import { createFileRoute } from "@tanstack/react-router";

import { IconButton, IconChevronDown } from "@left-curve/applets-kit";
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
  beforeLoad: async () => {
    const image = new Image();
    image.src = "/images/characters/group.svg";
    await image.decode();
  },
  component: OverviewComponent,
});

function OverviewComponent() {
  const [cardMobileVisible, setCardMobileVisible] = useState(0);

  const topYieldsRef = useRef<HTMLDivElement>(null);

  const scrollToSection = () => {
    topYieldsRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  return (
    <div className="w-full lg:max-w-[76rem] mx-auto flex flex-col gap-8 p-4 pb-32">
      <div className="w-full flex flex-col gap-8 min-h-[100dvh] lg:min-h-fit relative">
        <WelcomeSection
          cardMobileVisible={cardMobileVisible}
          setCardMobileVisible={setCardMobileVisible}
        />

        <DotsIndicator cardVisible={cardMobileVisible} setCardVisible={setCardMobileVisible} />

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
