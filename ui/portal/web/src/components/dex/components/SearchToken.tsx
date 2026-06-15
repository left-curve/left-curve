import { useAppConfig, useBoostedPairs, useCurrentEpoch, useFavPairs } from "@left-curve/store";
import { useMemo, useRef, useState } from "react";

import { IconSearch, Input, Popover, Tab, Tabs, useMediaQuery } from "@left-curve/applets-kit";
import { Sheet } from "react-modal-sheet";
import { SearchTokenTable } from "./SearchTokenTable";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { Image } from "~/components/foundation/Image";
import { MarketPair } from "@left-curve/foundation/market-pair";

import type { PopoverRef } from "@left-curve/applets-kit";
import type { GetAppConfigData } from "@left-curve/store";
import type React from "react";

function assertNever(x: never): never {
  throw new Error(`Unexpected value: ${String(x)}`);
}

type SearchTokenHeaderProps = {
  pair: MarketPair;
  isOpen?: boolean;
};

const SearchTokenHeader: React.FC<SearchTokenHeaderProps> = ({ pair }) => {
  return (
    <div className="flex gap-2 items-center">
      <Image src={pair.logoURI} alt={pair.base.symbol} className="h-6 w-6 drag-none select-none" />
      <p className="diatype-lg-heavy text-ink-secondary-700 min-w-fit">{pair.ticker}</p>
    </div>
  );
};

export type SearchTokenRow = {
  pair: MarketPair;
  isFavorite: boolean;
  /** Perps weight multiplier (e.g. "2.000000") when this pair is currently
   * boosted; undefined otherwise. */
  boostMultiplier?: string;
};

function normalizeRows(config: GetAppConfigData | undefined): SearchTokenRow[] {
  const rows: SearchTokenRow[] = [];

  // Pair availability comes from async app config; keep static metadata cataloged separately.
  const configuredPairIds = Object.keys(config?.perpsPairs ?? {});
  for (const pairId of configuredPairIds) {
    const pair = MarketPair.fromPairId(pairId);

    rows.push({
      pair,
      isFavorite: false,
    });
  }

  return rows;
}

type SearchTokenTab = "all" | "favorites" | "crypto" | "commodities";

const SearchTokenMenu: React.FC<{
  onChangePair: (row: SearchTokenRow) => void;
}> = ({ onChangePair }) => {
  const [activeTab, setActiveTab] = useState<SearchTokenTab>("all");
  const [searchText, setSearchText] = useState<string>("");
  const { data: config } = useAppConfig();
  const { hasFavPair, favPairs } = useFavPairs();
  const pointsUrl = window.dango.urls.pointsUrl;
  const { currentEpoch } = useCurrentEpoch({ pointsUrl });
  const { boostByPairId } = useBoostedPairs({ pointsUrl, currentEpoch });

  const allRows = useMemo(() => normalizeRows(config), [config]);

  const filteredRows = useMemo(
    () =>
      allRows
        .filter((row) => {
          switch (activeTab) {
            case "all":
              return true;
            case "crypto":
              return row.pair.type === "crypto";
            case "commodities":
              return row.pair.type === "commodity";
            case "favorites":
              return hasFavPair(row.pair.ticker);
            default:
              return assertNever(activeTab);
          }
        })
        .filter((row) => {
          if (!searchText) return true;
          const upper = searchText.toUpperCase();
          return (
            row.pair.ticker.toUpperCase().includes(upper) ||
            row.pair.name.toUpperCase().includes(upper)
          );
        })
        .map((row) => ({
          ...row,
          isFavorite: hasFavPair(row.pair.ticker),
          boostMultiplier: boostByPairId[row.pair.id],
        })),
    [allRows, activeTab, searchText, hasFavPair, boostByPairId],
  );

  const showFavoritesEmpty = activeTab === "favorites" && favPairs.length === 0;

  return (
    <div className="flex flex-col gap-2">
      <Input
        fullWidth
        startContent={<IconSearch className="w-5 h-5 text-ink-tertiary-500" />}
        value={searchText}
        onChange={(e) => setSearchText(e.target.value)}
        placeholder={
          <div className="flex gap-1 items-center">
            <p className="text-ink-tertiary-500 diatype-m-regular mt-[2px]">
              {m["dex.searchFor"]()}
            </p>
            <p className="exposure-m-italic text-ink-secondary-rice">{m["dex.tokens"]()}</p>
          </div>
        }
      />
      <div className="relative overflow-x-auto scrollbar-none pt-1">
        <Tabs
          color="line-red"
          layoutId="search-token-tabs"
          selectedTab={activeTab}
          onTabChange={(tab) => setActiveTab(tab as SearchTokenTab)}
          classNames={{ base: "z-10" }}
        >
          <Tab title="all">{m["dex.protrade.searchPairTable.tabs.all"]()}</Tab>
          <Tab title="favorites">{m["dex.protrade.searchPairTable.tabs.favorites"]()}</Tab>
          <Tab title="crypto">{m["dex.protrade.searchPairTable.tabs.crypto"]()}</Tab>
          <Tab title="commodities">{m["dex.protrade.searchPairTable.tabs.commodities"]()}</Tab>
        </Tabs>
        <span className="w-full absolute h-[2px] bg-outline-secondary-gray bottom-[0px] z-0" />
      </div>
      {showFavoritesEmpty ? (
        <p className="diatype-sm-medium text-ink-tertiary-500 text-center py-8">
          {m["dex.protrade.searchPairTable.emptyFavorites"]()}
        </p>
      ) : (
        <SearchTokenTable data={filteredRows} onChangePair={onChangePair} />
      )}
    </div>
  );
};

type SearchTokenProps = {
  pair: MarketPair;
  onChangePair: (row: SearchTokenRow) => void;
};

export const SearchToken: React.FC<SearchTokenProps> = ({ pair, onChangePair }) => {
  const { isLg } = useMediaQuery();
  const [isSearchTokenVisible, setIsSearchTokenVisible] = useState<boolean>(false);
  const popoverRef = useRef<PopoverRef>(null);

  if (isLg)
    return (
      <Popover
        showArrow={isLg}
        ref={popoverRef}
        classNames={{ menu: "min-w-[45rem]" }}
        trigger={<SearchTokenHeader pair={pair} isOpen={isSearchTokenVisible} />}
        menu={
          <SearchTokenMenu
            onChangePair={(row) => {
              popoverRef.current?.close();
              onChangePair(row);
            }}
          />
        }
      />
    );

  return (
    <>
      <div onClick={() => setIsSearchTokenVisible(true)} className="cursor-pointer">
        <SearchTokenHeader pair={pair} isOpen={isSearchTokenVisible} />
      </div>
      <Sheet
        isOpen={isSearchTokenVisible}
        onClose={() => setIsSearchTokenVisible(false)}
        rootId="root"
      >
        <Sheet.Container className="!bg-surface-primary-rice !rounded-t-2xl !shadow-none">
          <Sheet.Header />
          <Sheet.Content>
            <div className="flex flex-col gap-4 p-4">
              <SearchTokenMenu
                onChangePair={(row) => {
                  setIsSearchTokenVisible(false);
                  onChangePair(row);
                }}
              />
            </div>
          </Sheet.Content>
        </Sheet.Container>
        <Sheet.Backdrop onTap={() => setIsSearchTokenVisible(false)} />
      </Sheet>
    </>
  );
};
