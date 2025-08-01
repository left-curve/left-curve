import { useAccount } from "@left-curve/store";
import { useRef, useState } from "react";

import { createFileRoute } from "@tanstack/react-router";
import { m } from "~/paraglide/messages";

import { IconButton, IconChevronDown } from "@left-curve/applets-kit";
import { AppletsSection } from "~/components/overview/AppletsSection";
import { DotsIndicator } from "~/components/overview/SwippeableAccountCard";
import { WelcomeSection } from "~/components/overview/WelcomeSection";

export const Route = createFileRoute("/(app)/_app/")({
  head: () => ({
    meta: [{ title: `Dango | ${m["common.overview"]()}` }],
  }),
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
        className="bg-surface-secondary-rice shadow-account-card flex flex-col rounded-xl w-full pt-4"
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
