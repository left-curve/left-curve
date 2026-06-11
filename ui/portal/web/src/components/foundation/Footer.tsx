import {
  FormattedNumber,
  IconDiscord,
  IconTwitter,
  Link,
  Marquee,
  twMerge,
} from "@left-curve/applets-kit";
import { useAllPerpsPairStats } from "@left-curve/store";
import { Decimal } from "@left-curve/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useRouter } from "@tanstack/react-router";
import { StatusBadge } from "./StatusBadge";
import { memo } from "react";
import { getPerpsPairTicker } from "../dex/helpers/tradePairSymbols";

import type { NormalizedPerpsPairStats } from "@left-curve/store";

const TICKER_PRICE_CHANGE_FORMAT_OPTIONS = { fractionDigits: 2 };

function Footer() {
  const router = useRouter();
  const isTradeRoute = router.state.location.pathname.includes("trade");

  return (
    <footer
      className={twMerge(
        "hidden lg:flex fixed bottom-0 left-0 right-0 z-50 items-center gap-2 px-4 py-2 border-t border-outline-secondary-gray bg-surface-primary-rice",
        isTradeRoute && "shadow-account-card",
      )}
    >
      <div className="flex flex-1 items-center gap-2 min-w-0">
        <StatusBadge className="static flex" />

        <FooterTicker />
      </div>

      <div className="h-[17px] w-px bg-outline-secondary-gray shrink-0" />

      <div className="flex items-center gap-1 shrink-0">
        <Link
          href="/documents/Dango - Terms of Use.pdf"
          target="_blank"
          rel="noopener noreferrer"
          className="exposure-xs-italic"
        >
          {m["footer.terms"]()}
        </Link>
        <Link
          href="/documents/Dango - Privacy Policy.pdf"
          target="_blank"
          rel="noopener noreferrer"
          className="exposure-xs-italic"
        >
          {m["footer.privacyPolicy"]()}
        </Link>
        <Link
          href="https://discord.gg/BWJtyySxBM"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Discord"
        >
          <IconDiscord className="w-5 h-5" />
        </Link>
        <Link
          href="https://x.com/dango"
          target="_blank"
          rel="noopener noreferrer"
          aria-label="Twitter"
        >
          <IconTwitter className="w-5 h-5" />
        </Link>
      </div>
    </footer>
  );
}

const FooterTicker = memo(function FooterTicker() {
  const perpsPairStats = useAllPerpsPairStats((s) => s.perpsPairStats);

  return (
    <Marquee
      className="flex-1 min-w-0"
      direction="left"
      speed={40}
      item={
        <div className="flex items-center gap-3 pr-3">
          {perpsPairStats.map((stats) => (
            <TickerItem key={stats.pairId} stats={stats} />
          ))}
        </div>
      }
    />
  );
});

type TickerItemProps = {
  stats: NormalizedPerpsPairStats;
};

const TickerItem = memo(function TickerItem({ stats }: TickerItemProps) {
  const priceChange = stats.priceChange24H ? Decimal(stats.priceChange24H) : null;
  const isPositive = priceChange ? priceChange.gte(0) : true;

  return (
    <div className="flex items-center gap-1 diatype-xs-regular shrink-0 w-[8.5rem]">
      <span className="text-ink-secondary-700">{getPerpsPairTicker(stats.pairId)}</span>
      <span className={isPositive ? "text-utility-success-600" : "text-utility-error-600"}>
        {priceChange ? `${isPositive ? "+" : ""}` : ""}
        {priceChange ? (
          <FormattedNumber
            number={priceChange.toString()}
            formatOptions={TICKER_PRICE_CHANGE_FORMAT_OPTIONS}
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
});

export { Footer };
