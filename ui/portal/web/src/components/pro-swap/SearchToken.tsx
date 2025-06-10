import {
  Cell,
  IconChevronDownFill,
  IconSearch,
  Input,
  Popover,
  Table,
  type TableColumn,
  Tabs,
  useMediaQuery,
} from "@left-curve/applets-kit";
import type React from "react";
import { useState } from "react";
import { Sheet } from "react-modal-sheet";

const data = [
  {
    name: "BTC-USD",
    isFavorite: true,
    lastPrice: 45000,
    change: 2.5,
    "8hourChange": "-0.0100%",
    volume: "$2,227,275,754",
    openInterest: "$2,227,275,754",
  },
  {
    name: "ETH-USD",
    isFavorite: false,
    lastPrice: 45000,
    change: -0.2,
    "8hourChange": "-0.0100%",
    volume: "$2,227,275,754",
    openInterest: "$2,227,275,754",
  },
  {
    name: "FARTCOIN-USD",
    lastPrice: 45000,
    change: 2.5,
    "8hourChange": "-0.0100%",
    volume: "$2,227,275,754",
    openInterest: "$2,227,275,754",
  },
];

const columns: TableColumn<{
  name: string;
  lastPrice: number;
  change: number;
  "8hourChange": string;
  volume: string;
  openInterest: string;
}> = [
  {
    header: "Name",
    cell: ({ row }) => <Cell.Text text={row.original.name} />,
  },
  {
    header: "Last Price",
    cell: ({ row }) => <Cell.Text text={`$${row.original.lastPrice}`} />,
  },
  {
    header: "Change",
    cell: ({ row }) => (
      <Cell.Text
        text={`${row.original.change}%`}
        className={row.original.change > 0 ? "text-green-500" : "text-red-500"}
      />
    ),
  },
  {
    header: "8h Change",
    cell: ({ row }) => <Cell.Text text={row.original["8hourChange"]} />,
  },
  {
    header: "Volume",
    cell: ({ row }) => <Cell.Text text={row.original.volume} />,
  },
  {
    header: "Open Interest",
    cell: ({ row }) => <Cell.Text text={row.original.openInterest} />,
  },
];

type activeFilterType =
  | "All"
  | "Spot"
  | "Trending"
  | "DEX only"
  | "Pre-launch"
  | "AI"
  | "Defi"
  | "Gaming"
  | "Layer 1"
  | "Layer 2"
  | "Meme";

const SearchTokenHeader: React.FC = () => {
  return (
    <div className="flex gap-2 items-center">
      <img
        src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
        alt=""
        className="h-7 w-7 drag-none select-none"
      />
      <p className="diatype-lg-heavy text-gray-700 min-w-fit">ETH-USDC</p>
      <IconChevronDownFill className="text-gray-500 w-4 h-4 transition-all" />
    </div>
  );
};

const SearchTokenMenu: React.FC = () => {
  const [activeFilter, setActiveFilter] = useState<activeFilterType>("All");
  return (
    <div className="flex flex-col gap-2">
      <Input
        fullWidth
        startContent={<IconSearch className="w-5 h-5 text-gray-500" />}
        placeholder={
          <div className="flex gap-1 items-center">
            <p className="text-gray-500 diatype-m-regular mt-[2px]">Search for</p>
            <p className="exposure-m-italic text-rice-700">tokens</p>
          </div>
        }
      />
      <div className="relative overflow-x-auto scrollbar-none">
        <Tabs
          color="line-red"
          layoutId="tabs-open-order"
          selectedTab={activeFilter}
          keys={[
            "All",
            "Spot",
            "Trending",
            "DEX only",
            "Pre-launch",
            "AI",
            "Defi",
            "Gaming",
            "Layer 1",
            "Layer 2",
            "Meme",
          ]}
          onTabChange={(tab) => setActiveFilter(tab as activeFilterType)}
        />

        <span className="w-full absolute h-[1px] bg-gray-100 bottom-[0.25rem]" />
      </div>
      <Table data={data} columns={columns} style="simple" classNames={{ cell: "py-2" }} />
    </div>
  );
};

export const SearchToken: React.FC = () => {
  const { isLg } = useMediaQuery();
  const [isSearchTokenVisible, setIsSearchTokenVisible] = useState<boolean>(false);

  if (isLg)
    return (
      <Popover
        classNames={{ menu: "min-w-[45rem]" }}
        showArrow={false}
        trigger={<SearchTokenHeader />}
        menu={<SearchTokenMenu />}
      />
    );

  return (
    <>
      <div onClick={() => setIsSearchTokenVisible(true)} className="cursor-pointer">
        <SearchTokenHeader />
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
              <SearchTokenMenu />
            </div>
          </Sheet.Content>
        </Sheet.Container>
        <Sheet.Backdrop onTap={() => setIsSearchTokenVisible(false)} />
      </Sheet>
    </>
  );
};
