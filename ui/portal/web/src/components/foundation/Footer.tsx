import { FormattedNumber, twMerge } from "@left-curve/applets-kit";
import { allPerpsPairStatsStore, useAllPerpsPairStats } from "@left-curve/store";
import { Decimal } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useRouter } from "@tanstack/react-router";
import { StatusBadge } from "./StatusBadge";

import type { NormalizedPerpsPairStats } from "@left-curve/store";

const CURRENT_YEAR = new Date().getFullYear();

function Footer() {
  const router = useRouter();
  const isTradeRoute = router.state.location.pathname.includes("trade");

  useAllPerpsPairStats();
  const perpsPairStats = allPerpsPairStatsStore((s) => s.perpsPairStats);

  return (
    <footer
      className={twMerge(
        "hidden lg:flex fixed bottom-0 left-0 right-0 z-50 items-center gap-2 px-4 py-2 border-t border-outline-secondary-gray bg-surface-primary-rice",
        isTradeRoute && "shadow-account-card",
      )}
    >
      <div className="flex flex-1 items-center gap-2 min-w-0">
        <StatusBadge className="static flex" />

        <div className="flex items-center gap-3 overflow-hidden min-w-0">
          {perpsPairStats.map((stats) => (
            <TickerItem key={stats.pairId} stats={stats} />
          ))}
        </div>
      </div>

      <div className="h-[17px] w-px bg-outline-secondary-gray shrink-0" />

      <div className="flex items-center gap-1 shrink-0">
        <span className="exposure-xs-italic text-primitives-blue-light-500 px-1">
          Dango &copy; {CURRENT_YEAR}
        </span>
        <a
          href="/documents/Dango - Terms of Use.pdf"
          target="_blank"
          rel="noopener noreferrer"
          className="exposure-xs-italic text-primitives-blue-light-500 px-1 hover:opacity-80"
        >
          {m["footer.terms"]()}
        </a>
        <a
          href="/documents/Dango - Privacy Policy.pdf"
          target="_blank"
          rel="noopener noreferrer"
          className="exposure-xs-italic text-primitives-blue-light-500 px-1 hover:opacity-80"
        >
          {m["footer.privacyPolicy"]()}
        </a>
      </div>
    </footer>
  );
}

type TickerItemProps = {
  stats: NormalizedPerpsPairStats;
};

function TickerItem({ stats }: TickerItemProps) {
  const priceChange = stats.priceChange24H ? Decimal(stats.priceChange24H) : null;
  const isPositive = priceChange ? priceChange.gte(0) : true;

  return (
    <div className="flex items-center gap-1 diatype-xs-regular shrink-0 w-[8.5rem]">
      <span className="text-ink-secondary-700">
        {stats.pairId.replace("perp/", "").toUpperCase()}
      </span>
      <span className={isPositive ? "text-utility-success-600" : "text-utility-error-600"}>
        {priceChange ? `${isPositive ? "+" : ""}` : ""}
        {priceChange ? (
          <FormattedNumber
            number={priceChange.toString()}
            formatOptions={{ fractionDigits: 2 }}
            as="span"
          />
        ) : (
          "-"
        )}
        {priceChange ? "%" : ""}
      </span>
      <span className="text-ink-placeholder-400">
        {stats.currentPrice ? <FormattedNumber number={stats.currentPrice} as="span" /> : "-"}
      </span>
    </div>
  );
}

export { Footer };
