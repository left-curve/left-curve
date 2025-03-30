import { createContext, useContext } from "react";

import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import {
  IconCopy,
  Skeleton,
  Table,
  type TableColumn,
  TruncateText,
  twMerge,
  useCountdown,
  useMediaQuery,
} from "@left-curve/applets-kit";

import { m } from "~/paraglide/messages";

import type { IndexedBlock, IndexedTransaction } from "@left-curve/dango/types";
import type { UseQueryResult } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import type React from "react";
import type { PropsWithChildren } from "react";
import { HeaderExplorer } from "./HeaderExplorer";

type BlockExplorerProps = {
  height: string;
  className?: string;
};

const BlockExplorerContext = createContext<
  (UseQueryResult<IndexedBlock | null> & { height: string }) | null
>(null);

const useBlockExplorer = () => {
  const context = useContext(BlockExplorerContext);
  if (!context) {
    throw new Error("useBlockExplorer must be used within a BlockExplorerProvider");
  }
  return context;
};

const BlockContainer: React.FC<PropsWithChildren<BlockExplorerProps>> = ({
  height,
  children,
  className,
}) => {
  const client = usePublicClient();

  const query = useQuery({
    queryKey: ["block", height],
    queryFn: () => client.queryBlock({ height: +height }),
  });
  return (
    <BlockExplorerContext.Provider value={{ ...query, height }}>
      <div
        className={twMerge("w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16", className)}
      >
        {children}
      </div>
    </BlockExplorerContext.Provider>
  );
};

const BlockSkeleton: React.FC = () => {
  const { isLoading } = useBlockExplorer();

  if (!isLoading) return null;

  return (
    <div className="w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16">
      <div className="flex flex-col gap-4 rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow text-gray-700 diatype-m-bold relative overflow-hidden md:min-h-[147.22px] min-h-[208.5px]">
        <h1 className="h4-bold">{m["explorer.block.details.blockDetails"]()}</h1>
        <Skeleton className="h-full w-full max-w-[75%]" />
        <img
          src="/images/emojis/detailed/map-explorer.svg"
          alt="map-emoji"
          className="hidden md:block w-[16.25rem] h-[16.25rem] opacity-40 absolute top-[-2rem] right-[2rem] mix-blend-multiply"
        />
      </div>
    </div>
  );
};

const fakeDate = +new Date() * 1 + 1000 * 60 * 60 * 24;

const FutureBlock: React.FC = () => {
  const { height, data, isLoading } = useBlockExplorer();
  const { days, hours, minutes, seconds } = useCountdown(fakeDate);

  if (isLoading || data || Number.isNaN(Number(height))) return null;

  return (
    <div className="w-full md:max-w-[76rem] p-4 flex flex-col gap-6">
      <div className="flex flex-col gap-6 rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow relative overflow-hidden text-gray-700">
        <div className="flex flex-col gap-1">
          <h3 className="h4-heavy text-gray-900">
            {m["explorer.block.futureBlock.targetBlock"]()} {height}
          </h3>
          <p className="diatype-m-medium text-gray-500">
            {m["explorer.block.futureBlock.description"]()}
          </p>
        </div>
        <div className="w-full lg:max-w-[45.5rem] flex gap-4 flex-col lg:flex-row lg:items-center justify-between">
          <div className="flex flex-col gap-1">
            <p className="diatype-m-medium text-gray-500">
              {m["explorer.block.futureBlock.estimateTimeISO"]()}
            </p>
            <p className="diatype-m-bold text-gray-700">{new Date(fakeDate).toISOString()}</p>
          </div>
          <div className="flex flex-col gap-1">
            <p className="diatype-m-medium text-gray-500">
              {m["explorer.block.futureBlock.estimateTimeUTC"]()}
            </p>
            <p className="diatype-m-bold text-gray-700">{new Date(fakeDate).toUTCString()}</p>
          </div>
        </div>
        <span className="w-full h-[1px] bg-gray-200 lg:max-w-[45.5rem]" />
        <div className="grid grid-cols-3 lg:grid-cols-7 gap-3 items-center text-center lg:max-w-[45.5rem]">
          <div>
            <p className="h1-bold text-gray-900">{days}</p>
            <span className="diatype-m-medium uppercase text-gray-500">
              {m["countdown.days"]({ days })}
            </span>
          </div>
          <span className="h1-bold text-gray-900">:</span>
          <div>
            <p className="h1-bold text-gray-900">{hours}</p>
            <span className="diatype-m-medium uppercase text-gray-500">
              {m["countdown.hours"]({ hours })}
            </span>
          </div>
          <span className="hidden lg:flex h1-bold text-gray-900">:</span>
          <div>
            <p className="h1-bold text-gray-900">{minutes}</p>
            <span className="diatype-m-medium uppercase text-gray-500">
              {m["countdown.minutes"]({ minutes })}
            </span>
          </div>
          <span className="h1-bold text-gray-900">:</span>
          <div>
            <p className="h1-bold text-gray-900">{seconds}</p>
            <span className="diatype-m-medium uppercase text-gray-500">
              {m["countdown.seconds"]({ seconds })}
            </span>
          </div>
        </div>
        <img
          src="/images/emojis/detailed/map-explorer.svg"
          alt="map-emoji"
          className="w-[16.25rem] h-[16.25rem] opacity-40 absolute right-[2rem] mix-blend-multiply hidden lg:flex"
        />
      </div>
      <HeaderExplorer>
        <div className="flex flex-col gap-2 items-center w-full">
          <h3 className="exposure-m-italic text-gray-700">
            {m["explorer.block.futureBlock.hasNotBeenCreated"]({ height })}
          </h3>
          <div className="flex items-center justify-around gap-4 flex-col lg:flex-row w-full">
            <div className="flex flex-col gap-1 items-center">
              <p className="diatype-m-medium text-gray-500">
                {m["explorer.block.futureBlock.targetBlock"]()}
              </p>
              <p className="diatype-m-bold text-gray-700">#{height}</p>
            </div>
            <span className="w-full h-[1px] max-w-44 lg:w-[1px] lg:h-9 bg-gray-200" />
            <div className="flex flex-col gap-1 items-center">
              <p className="diatype-m-medium text-gray-500">
                {m["explorer.block.futureBlock.currentBlock"]()}
              </p>
              <p className="diatype-m-bold text-gray-700">#-</p>
            </div>
            <span className="w-full h-[1px] max-w-44 lg:w-[1px] lg:h-9 bg-gray-200" />
            <div className="flex flex-col gap-1 items-center">
              <p className="diatype-m-medium text-gray-500">
                {m["explorer.block.futureBlock.remainingBlocks"]()}
              </p>
              <p className="diatype-m-bold text-gray-700">#-</p>
            </div>
          </div>
        </div>
      </HeaderExplorer>
    </div>
  );
};

const BlockDetails: React.FC = () => {
  const { isMd } = useMediaQuery();
  const { data: blockInfo } = useBlockExplorer();
  if (!blockInfo) return null;

  console.log(blockInfo);

  const { transactions, createdAt, blockHeight, hash } = blockInfo;
  return (
    <div className="flex flex-col rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow text-gray-700 diatype-m-bold relative overflow-hidden">
      <div className="overflow-y-auto scrollbar-none w-full gap-4 flex flex-col">
        <h1 className="h4-bold">{m["explorer.block.details.blockDetails"]()}</h1>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
          <div className="col-span-1 md:col-span-2 flex items-center gap-1">
            <p className="diatype-md-medium text-gray-500">
              {m["explorer.block.details.blockHash"]()}
            </p>
            {isMd ? <p>{hash}</p> : <TruncateText text={hash} />}
            <IconCopy className="w-4 h-4 cursor-pointer" copyText={hash} />
          </div>
          <div className="flex items-center gap-1">
            <p className="diatype-md-medium text-gray-500">
              {m["explorer.block.details.blockHeight"]()}
            </p>
            <p>{blockHeight}</p>
          </div>
          <div className="flex items-center gap-1">
            <p className="diatype-md-medium text-gray-500">
              {m["explorer.block.details.proposer"]()}
            </p>
            <p>Leftcurve Validator</p>
          </div>
          <div className="flex items-center gap-1">
            <p className="diatype-md-medium text-gray-500">
              {m["explorer.block.details.numberOfTx"]()}
            </p>
            <p>{transactions.length}</p>
          </div>
          <div className="flex items-center gap-1">
            <p className="diatype-md-medium text-gray-500">
              {m["explorer.block.details.blockTime"]()}
            </p>
            <p>{new Date(createdAt).toISOString()}</p>
          </div>
        </div>
        {isMd ? (
          <img
            src="/images/emojis/detailed/map-explorer.svg"
            alt="map-emoji"
            className="w-[16.25rem] h-[16.25rem] opacity-40 absolute top-[-2rem] right-[2rem] mix-blend-multiply"
          />
        ) : null}
      </div>
    </div>
  );
};

const BlockNotFound: React.FC = () => {
  const { isLoading, data, height } = useBlockExplorer();

  if (!isLoading && !data && Number.isNaN(Number(height))) {
    return (
      <HeaderExplorer>
        <div className="flex flex-col gap-2 items-center border border-red-bean-50">
          <h3 className="exposure-m-italic text-gray-700">
            {m["explorer.block.notFound.title"]()}
          </h3>
          <p className="diatype-m-medium max-w-[42.5rem] text-center text-gray-500 ">
            {m["explorer.block.notFound.description"]()}
          </p>
        </div>
      </HeaderExplorer>
    );
  }

  return null;
};

const BlockTable: React.FC = () => {
  const navigate = useNavigate();
  const { data: blockInfo } = useBlockExplorer();
  if (!blockInfo) return null;

  const { transactions } = blockInfo;

  const columns: TableColumn<IndexedTransaction> = [
    {
      header: "Type",
      cell: ({ row }) => <p>{row.original.transactionType}</p>,
    },
    {
      header: "Hash",
      cell: ({ row }) => (
        <TruncateText
          className="cursor-pointer"
          text={row.original.hash}
          onClick={() => navigate({ to: `/tx/${row.original.hash}` })}
        />
      ),
    },
    {
      header: "Account",
      cell: ({ row }) => <p>{row.original.sender}</p>,
    },
    {
      header: "Result",
      cell: ({ row }) => {
        const { hasSucceeded } = row.original;
        return (
          <p className={hasSucceeded ? "text-status-success" : "text-status-fail"}>
            {hasSucceeded ? "Success" : "Fail"}
          </p>
        );
      },
    },
  ];

  return transactions.length ? <Table data={transactions} columns={columns} /> : null;
};

export const BlockExplorer = Object.assign(BlockContainer, {
  Skeleton: BlockSkeleton,
  FutureBlock: FutureBlock,
  NotFound: BlockNotFound,
  Details: BlockDetails,
  TxTable: BlockTable,
});
