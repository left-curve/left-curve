import { useAppConfig, useConfig, perpsMarginAsset, useFavPairs } from "@left-curve/store";
import { useRef, useState } from "react";

import { IconSearch, Input, Popover, Tabs, useMediaQuery } from "@left-curve/applets-kit";
import { Sheet } from "react-modal-sheet";
import { SearchTokenTable } from "./SearchTokenTable";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { PopoverRef } from "@left-curve/applets-kit";
import type { PairId, PerpsPairParam } from "@left-curve/types";
import type { AnyCoin } from "@left-curve/store/types";
import type React from "react";

type SearchTokenHeaderProps = {
  pairId: PairId;
  isOpen?: boolean;
};

const SearchTokenHeader: React.FC<SearchTokenHeaderProps> = ({ pairId }) => {
  const { coins } = useConfig();

  const baseCoin = coins.byDenom[pairId.baseDenom];
  const quoteCoin = { symbol: perpsMarginAsset.symbol, logoURI: perpsMarginAsset.logoURI };

  return (
    <div className="flex gap-2 items-center">
      <img
        src={baseCoin?.logoURI}
        alt={baseCoin?.symbol}
        className="h-6 w-6 drag-none select-none"
      />
      <p className="diatype-lg-heavy text-ink-secondary-700 min-w-fit">
        {`${baseCoin?.symbol ?? "?"}-${quoteCoin?.symbol ?? "?"}`}
      </p>
    </div>
  );
};

export type SearchTokenRow = {
  baseCoin: AnyCoin;
  quoteCoin: AnyCoin;
  pairId: PairId;
  pairKey: string;
  perpsPairId: string;
};

function normalizeRows(
  config: any,
  coins: { byDenom: Record<string, AnyCoin>; bySymbol: Record<string, AnyCoin> },
): SearchTokenRow[] {
  const rows: SearchTokenRow[] = [];

  const perpsPairs: Record<string, PerpsPairParam> = (config as any)?.perpsPairs ?? {};
  for (const [perpsPairId, _param] of Object.entries(perpsPairs)) {
    const symbol = perpsPairId.replace("perp/", "").toUpperCase();

    const baseSym = symbol.replace(/USDC$|USD$/, "");
    const base = coins.bySymbol[baseSym];
    if (!base) continue;

    const syntheticQuote: AnyCoin = {
      symbol: perpsMarginAsset.symbol,
      denom: "usd",
      decimals: perpsMarginAsset.decimals,
      name: perpsMarginAsset.name,
      logoURI: perpsMarginAsset.logoURI,
      type: "native",
    };

    rows.push({
      baseCoin: base,
      quoteCoin: syntheticQuote,
      pairId: { baseDenom: base.denom, quoteDenom: "usd" },
      pairKey: `${base.symbol}-USD`,
      perpsPairId,
    });
  }

  return rows;
}

const SEARCH_TOKEN_TABS = ["All", "Favorites", "Crypto"] as const;
type SearchTokenTab = (typeof SEARCH_TOKEN_TABS)[number];

const SearchTokenMenu: React.FC<{
  pairId: PairId;
  onChangePairId: (row: SearchTokenRow) => void;
}> = ({ onChangePairId }) => {
  const [activeTab, setActiveTab] = useState<SearchTokenTab>("All");
  const [searchText, setSearchText] = useState<string>("");
  const { data: config } = useAppConfig();
  const { coins } = useConfig();
  const { hasFavPair, favPairs } = useFavPairs();

  const allRows = normalizeRows(config, coins);

  const filteredRows = allRows
    .filter((row) => {
      switch (activeTab) {
        case "All":
        case "Crypto":
          return true;
        case "Favorites":
          return hasFavPair(row.pairKey);
      }
    })
    .filter((row) => {
      if (!searchText) return true;
      const upper = searchText.toUpperCase();
      return (
        row.baseCoin.symbol.toUpperCase().includes(upper) ||
        row.quoteCoin.symbol.toUpperCase().includes(upper) ||
        row.pairKey.toUpperCase().includes(upper)
      );
    });

  const showFavoritesEmpty = activeTab === "Favorites" && favPairs.length === 0;

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
          keys={[...SEARCH_TOKEN_TABS]}
          onTabChange={(tab) => setActiveTab(tab as SearchTokenTab)}
          classNames={{ base: "z-10" }}
        />
        <span className="w-full absolute h-[2px] bg-outline-secondary-gray bottom-[0px] z-0" />
      </div>
      {showFavoritesEmpty ? (
        <p className="diatype-sm-medium text-ink-tertiary-500 text-center py-8">
          You haven't added any favorites yet. Tap the star next to a pair to add it.
        </p>
      ) : (
        <SearchTokenTable data={filteredRows} onChangePairId={onChangePairId} />
      )}
    </div>
  );
};

type SearchTokenProps = {
  pairId: PairId;
  onChangePairId: (row: SearchTokenRow) => void;
};

export const SearchToken: React.FC<SearchTokenProps> = ({ pairId, onChangePairId }) => {
  const { isLg } = useMediaQuery();
  const [isSearchTokenVisible, setIsSearchTokenVisible] = useState<boolean>(false);
  const popoverRef = useRef<PopoverRef>(null);

  if (isLg)
    return (
      <Popover
        showArrow={isLg}
        ref={popoverRef}
        classNames={{ menu: "min-w-[45rem]" }}
        trigger={<SearchTokenHeader pairId={pairId} isOpen={isSearchTokenVisible} />}
        menu={
          <SearchTokenMenu
            pairId={pairId}
            onChangePairId={(row) => {
              popoverRef.current?.close();
              onChangePairId(row);
            }}
          />
        }
      />
    );

  return (
    <>
      <div onClick={() => setIsSearchTokenVisible(true)} className="cursor-pointer">
        <SearchTokenHeader pairId={pairId} isOpen={isSearchTokenVisible} />
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
                pairId={pairId}
                onChangePairId={(row) => {
                  setIsSearchTokenVisible(false);
                  onChangePairId(row);
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
