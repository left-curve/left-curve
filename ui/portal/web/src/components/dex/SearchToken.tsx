import { useAppConfig, useConfig } from "@left-curve/store";
import { useRef, useState } from "react";

import {
  IconChevronDownFill,
  IconSearch,
  Input,
  Popover,
  Tabs,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { Sheet } from "react-modal-sheet";
import { SearchTokenTable } from "./SearchTokenTable";

import { m } from "~/paraglide/messages";

import type { PopoverRef } from "@left-curve/applets-kit";
import type { PairId } from "@left-curve/dango/types";
import type React from "react";

type SearchTokenHeaderProps = {
  pairId: PairId;
};

const SearchTokenHeader: React.FC<SearchTokenHeaderProps> = ({ pairId }) => {
  const { coins } = useConfig();
  const baseCoin = coins[pairId.baseDenom];
  const quoteCoin = coins[pairId.quoteDenom];
  return (
    <div className="flex gap-2 items-center">
      <img src={baseCoin.logoURI} alt={baseCoin.symbol} className="h-6 w-6 drag-none select-none" />
      <p className="diatype-lg-heavy text-gray-700 min-w-fit">{`${baseCoin.symbol}-${quoteCoin.symbol}`}</p>
      <IconChevronDownFill className="text-gray-500 w-4 h-4 transition-all" />
    </div>
  );
};

const SearchTokenMenu: React.FC<SearchTokenProps> = ({ pairId, onChangePairId }) => {
  const [activeFilter, setActiveFilter] = useState<string>("All");
  const [searchText, setSearchText] = useState<string>("");
  const { data: config } = useAppConfig();

  return (
    <div className="flex flex-col gap-2">
      <Input
        fullWidth
        startContent={<IconSearch className="w-5 h-5 text-gray-500" />}
        value={searchText}
        onChange={(e) => setSearchText(e.target.value)}
        placeholder={
          <div className="flex gap-1 items-center">
            <p className="text-gray-500 diatype-m-regular mt-[2px]">{m["dex.searchFor"]()}</p>
            <p className="exposure-m-italic text-rice-700">{m["dex.tokens"]()}</p>
          </div>
        }
      />
      <div className="relative overflow-x-auto scrollbar-none">
        <Tabs
          color="line-red"
          layoutId="search-token-tabs"
          selectedTab={activeFilter}
          keys={["All", "Spot"]}
          onTabChange={setActiveFilter}
        />

        <span className="w-full absolute h-[1px] bg-gray-100 bottom-[0.25rem]" />
      </div>
      <SearchTokenTable>
        <SearchTokenTable.Spot
          classNames={{ cell: "py-2" }}
          data={Object.values(config?.pairs || {})}
          searchText={searchText.toUpperCase()}
          onChangePairId={onChangePairId}
          pairId={pairId}
        />
      </SearchTokenTable>
    </div>
  );
};

type SearchTokenProps = {
  pairId: PairId;
  onChangePairId: (pairId: PairId) => void;
};

export const SearchToken: React.FC<SearchTokenProps> = ({ pairId, onChangePairId }) => {
  const { isLg } = useMediaQuery();
  const [isSearchTokenVisible, setIsSearchTokenVisible] = useState<boolean>(false);
  const popoverRef = useRef<PopoverRef>(null);

  if (isLg)
    return (
      <Popover
        ref={popoverRef}
        classNames={{ menu: "min-w-[45rem]" }}
        showArrow={false}
        trigger={<SearchTokenHeader pairId={pairId} />}
        menu={
          <SearchTokenMenu
            pairId={pairId}
            onChangePairId={(pairId) => {
              popoverRef.current?.close();
              onChangePairId(pairId);
            }}
          />
        }
      />
    );

  return (
    <>
      <div onClick={() => setIsSearchTokenVisible(true)} className="cursor-pointer">
        <SearchTokenHeader pairId={pairId} />
      </div>
      <Sheet
        isOpen={isSearchTokenVisible}
        onClose={() => setIsSearchTokenVisible(false)}
        rootId="root"
      >
        <Sheet.Container className="!bg-white-100 !rounded-t-2xl !shadow-none">
          <Sheet.Header />
          <Sheet.Content>
            <div className="flex flex-col gap-4 p-4">
              <SearchTokenMenu
                pairId={pairId}
                onChangePairId={(pairId) => {
                  setIsSearchTokenVisible(false);
                  // onChangePairId(pairId);
                  console.log("Selected pairId:", pairId);
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
