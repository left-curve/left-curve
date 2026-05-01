import {
  FormattedNumber,
  IconChecked,
  IconSliders,
  Popover,
  twMerge,
} from "@left-curve/applets-kit";
import { allPerpsPairStatsStore, useAllPerpsPairStats, useStorage } from "@left-curve/store";
import { Decimal } from "@left-curve/dango/utils";
import { useRouter } from "@tanstack/react-router";
import { StatusBadge } from "./StatusBadge";

import type { NormalizedPerpsPairStats } from "@left-curve/store";

type TickerDisplayMode = "popular-perp" | "hidden";

function Footer() {
  const router = useRouter();
  const isTradeRoute = router.state.location.pathname.includes("trade");

  const [tickerMode, setTickerMode] = useStorage<TickerDisplayMode>("footer-ticker-mode", {
    initialValue: "popular-perp",
  });

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
        <div className="flex items-center gap-4 shrink-0">
          <StatusBadge className="static flex" />
          <TickerModeDropdown tickerMode={tickerMode} onChangeMode={setTickerMode} />
        </div>

        {tickerMode === "popular-perp" && (
          <div className="flex items-center gap-3 overflow-hidden min-w-0">
            {perpsPairStats.map((stats) => (
              <TickerItem key={stats.pairId} stats={stats} />
            ))}
          </div>
        )}
      </div>

      <div className="h-[17px] w-px bg-outline-secondary-gray shrink-0" />

      <div className="flex items-center gap-1 shrink-0">
        <span className="exposure-xs-italic text-primitives-blue-light-500 px-1">
          Dango &copy; {new Date().getFullYear()}
        </span>
        <a
          href="/documents/Dango - Terms of Use.pdf"
          target="_blank"
          rel="noopener noreferrer"
          className="exposure-xs-italic text-primitives-blue-light-500 px-1 hover:opacity-80"
        >
          Terms
        </a>
        <a
          href="/documents/Dango - Privacy Policy.pdf"
          target="_blank"
          rel="noopener noreferrer"
          className="exposure-xs-italic text-primitives-blue-light-500 px-1 hover:opacity-80"
        >
          Privacy Policy
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

type TickerModeDropdownProps = {
  tickerMode: TickerDisplayMode;
  onChangeMode: (mode: TickerDisplayMode) => void;
};

function TickerModeDropdown({ tickerMode, onChangeMode }: TickerModeDropdownProps) {
  return (
    <Popover
      showArrow={false}
      anchor="top"
      trigger={
        <IconSliders className="w-4 h-4 text-ink-tertiary-500 cursor-pointer hover:text-ink-secondary-700 transition-colors" />
      }
      menu={
        <div className="flex flex-col py-2">
          <div className="px-1">
            <button
              type="button"
              className="flex items-center justify-between gap-3 p-2 rounded-lg hover:bg-surface-tertiary-rice cursor-pointer diatype-m-medium text-ink-secondary-700 w-full text-left"
              onClick={() => onChangeMode("popular-perp")}
            >
              Popular Perp
              {tickerMode === "popular-perp" && (
                <IconChecked className="w-3.5 h-3.5 text-utility-success-500" />
              )}
            </button>
          </div>
          <div className="px-1">
            <button
              type="button"
              className="flex items-center justify-between gap-3 p-2 rounded-lg hover:bg-surface-tertiary-rice cursor-pointer diatype-m-medium text-ink-secondary-700 w-full text-left"
              onClick={() => onChangeMode("hidden")}
            >
              Do Not Display
              {tickerMode === "hidden" && (
                <IconChecked className="w-3.5 h-3.5 text-utility-success-500" />
              )}
            </button>
          </div>
        </div>
      }
      classNames={{
        menu: "!rounded-[20px] p-0 min-w-[10rem]",
      }}
    />
  );
}

export { Footer };
