import { createContext, useContext, useEffect, useState } from "react";

import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import { Skeleton, TextCopy, twMerge, useCountdown, useWatchEffect } from "@left-curve/applets-kit";
import { HeaderExplorer } from "./HeaderExplorer";
import { TransactionsTable } from "./TransactionsTable";

import { m } from "~/paraglide/messages";

import type { IndexedBlock } from "@left-curve/dango/types";
import type { UseQueryResult } from "@tanstack/react-query";

import type React from "react";
import type { PropsWithChildren } from "react";

type BlockExplorerProps = {
  height: string;
  className?: string;
};

const BlockExplorerContext = createContext<UseQueryResult<{
  searchBlock: IndexedBlock | null;
  currentBlock: IndexedBlock;
  height: number;
  isFutureBlock: boolean;
  isInvalidBlock: boolean;
}> | null>(null);

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
    queryFn: async () => {
      const isLatest = height === "latest";
      const parsedHeight = Number(height);
      const [searchBlock, currentBlock] = await Promise.all([
        Number.isNaN(parsedHeight) && !isLatest
          ? null
          : client.queryBlock(isLatest ? undefined : { height: parsedHeight }),
        client.queryBlock(),
      ]);
      const isFutureBlock = parsedHeight > 0 && parsedHeight > currentBlock?.blockHeight;
      const isInvalidBlock = (!isLatest && Number.isNaN(parsedHeight)) || parsedHeight < 0;
      return { searchBlock, currentBlock, height: parsedHeight, isFutureBlock, isInvalidBlock };
    },
  });

  return (
    <BlockExplorerContext.Provider value={query}>
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
      <div className="flex flex-col gap-4 rounded-xl p-4 bg-surface-secondary-rice shadow-account-card text-secondary-700 diatype-m-bold relative overflow-hidden md:min-h-[177.63px] min-h-[208.5px]">
        <h1 className="h4-bold text-primary-900">
          {m["explorer.block.details.blockDetails"]({ height: "#" })}
        </h1>
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

const FutureBlock: React.FC = () => {
  const { data } = useBlockExplorer();
  const [countdown, setCountdown] = useState<number>(0);
  const [blockData, setBlockData] = useState<number>();
  const { days, hours, minutes, seconds } = useCountdown({ date: blockData });

  useWatchEffect(seconds, () => setCountdown((v) => v + 2));

  useEffect(() => {
    if (!data || !data.isFutureBlock) return;
    const { currentBlock, height } = data;
    const blockDiff = height - currentBlock.blockHeight;
    setBlockData(Date.now() + blockDiff * 500); // Assuming 500ms per block
  }, [data]);

  if (!data?.isFutureBlock || !blockData) return null;

  const { height, currentBlock } = data;
  const blockDiff = height - currentBlock.blockHeight;

  const getRemainingBlocks = () => {
    if (!blockDiff) return "-";
    const diff = blockDiff - countdown;
    return diff < 0 ? 0 : diff;
  };

  return (
    <div className="w-full md:max-w-[76rem] p-4 flex flex-col gap-6">
      <div className="flex flex-col gap-6 rounded-md p-4 bg-surface-secondary-rice shadow-account-card relative overflow-hidden text-secondary-700">
        <div className="flex flex-col gap-1">
          <h3 className="h4-heavy text-primary-900">
            {m["explorer.block.futureBlock.targetBlock"]()} {height}
          </h3>
          <p className="diatype-m-medium text-tertiary-500">
            {m["explorer.block.futureBlock.description"]()}
          </p>
        </div>
        <div className="w-full lg:max-w-[45.5rem] flex gap-4 flex-col lg:flex-row lg:items-center justify-between">
          <div className="flex flex-col gap-1">
            <p className="diatype-m-medium text-tertiary-500">
              {m["explorer.block.futureBlock.estimateTimeISO"]()}
            </p>
            <p className="diatype-m-bold text-secondary-700">
              {blockData ? new Date(blockData).toISOString() : "-"}
            </p>
          </div>
          <div className="flex flex-col gap-1">
            <p className="diatype-m-medium text-tertiary-500">
              {m["explorer.block.futureBlock.estimateTimeUTC"]()}
            </p>
            <p className="diatype-m-bold text-secondary-700">
              {blockData ? new Date(blockData).toUTCString() : "-"}
            </p>
          </div>
        </div>
        <span className="w-full h-[1px] bg-secondary-gray lg:max-w-[45.5rem]" />
        <div className="grid grid-cols-3 lg:grid-cols-7 gap-3 items-center text-center lg:max-w-[45.5rem]">
          <div>
            <p className="h1-bold text-primary-900">{days}</p>
            <span className="diatype-m-medium uppercase text-tertiary-500">
              {m["countdown.days"]({ days })}
            </span>
          </div>
          <span className="h1-bold text-primary-900">:</span>
          <div>
            <p className="h1-bold text-primary-900">{hours}</p>
            <span className="diatype-m-medium uppercase text-tertiary-500">
              {m["countdown.hours"]({ hours })}
            </span>
          </div>
          <span className="hidden lg:flex h1-bold text-primary-900">:</span>
          <div>
            <p className="h1-bold text-primary-900">{minutes}</p>
            <span className="diatype-m-medium uppercase text-tertiary-500">
              {m["countdown.minutes"]({ minutes })}
            </span>
          </div>
          <span className="h1-bold text-primary-900">:</span>
          <div>
            <p className="h1-bold text-primary-900">{seconds}</p>
            <span className="diatype-m-medium uppercase text-tertiary-500">
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
          <h3 className="exposure-m-italic text-secondary-700">
            {m["explorer.block.futureBlock.hasNotBeenCreated"]({ height })}
          </h3>
          <div className="flex items-center justify-around gap-4 flex-col lg:flex-row w-full">
            <div className="flex flex-col gap-1 items-center">
              <p className="diatype-m-medium text-tertiary-500">
                {m["explorer.block.futureBlock.targetBlock"]()}
              </p>
              <p className="diatype-m-bold text-secondary-700">#{height}</p>
            </div>
            <span className="w-full h-[1px] max-w-44 lg:w-[1px] lg:h-9 bg-secondary-gray" />
            <div className="flex flex-col gap-1 items-center">
              <p className="diatype-m-medium text-tertiary-500">
                {m["explorer.block.futureBlock.currentBlock"]()}
              </p>
              <p className="diatype-m-bold text-secondary-700">
                #{currentBlock.blockHeight ? currentBlock.blockHeight + countdown : "-"}
              </p>
            </div>
            <span className="w-full h-[1px] max-w-44 lg:w-[1px] lg:h-9 bg-secondary-gray" />
            <div className="flex flex-col gap-1 items-center">
              <p className="diatype-m-medium text-tertiary-500">
                {m["explorer.block.futureBlock.remainingBlocks"]()}
              </p>
              <p className="diatype-m-bold text-secondary-700">#{getRemainingBlocks()}</p>
            </div>
          </div>
        </div>
      </HeaderExplorer>
    </div>
  );
};

const BlockDetails: React.FC = () => {
  const { data } = useBlockExplorer();

  if (!data?.searchBlock) return null;

  const { transactions, createdAt, blockHeight, hash } = data.searchBlock;

  return (
    <div className="flex flex-col rounded-md p-4 bg-surface-secondary-rice shadow-account-card text-secondary-700 relative overflow-hidden diatype-sm-medium">
      <div className="overflow-y-auto scrollbar-none w-full gap-4 flex flex-col">
        <h1 className="h4-bold text-primary-900">
          {m["explorer.block.details.blockDetails"]({ height: `#${blockHeight}` })}
        </h1>
        <div className="grid grid-cols-1 gap-3 md:gap-2">
          <div className="flex md:items-center gap-1 flex-col md:flex-row">
            <p className="diatype-sm-medium text-tertiary-500 md:min-w-[8rem]">
              {m["explorer.block.details.blockHash"]()}
            </p>
            <p className="break-all whitespace-normal diatype-mono-sm-medium">
              {hash}
              <TextCopy
                className="inline-block align-middle ml-1 w-4 h-4 cursor-pointer"
                copyText={hash}
              />
            </p>
          </div>
          <div className="flex md:items-center gap-1 flex-col md:flex-row">
            <p className="diatype-sm-medium text-tertiary-500 md:min-w-[8rem]">
              {m["explorer.block.details.proposer"]()}
            </p>
            <p>Leftcurve Validator</p>
          </div>
          <div className="flex md:items-center gap-1 flex-col md:flex-row">
            <p className="diatype-sm-medium text-tertiary-500 md:min-w-[8rem]">
              {m["explorer.block.details.numberOfTx"]()}
            </p>
            <p>{transactions.length}</p>
          </div>
          <div className="flex md:items-center gap-1 flex-col md:flex-row">
            <p className="diatype-sm-medium text-tertiary-500 md:min-w-[8rem]">
              {m["explorer.block.details.blockTime"]()}
            </p>
            <p className="break-all whitespace-normal">{new Date(createdAt).toLocaleString()}</p>
          </div>
        </div>
        <img
          src="/images/emojis/detailed/map-explorer.svg"
          alt="map-emoji"
          className="w-[16.25rem] h-[16.25rem] opacity-40 absolute top-[-2rem] right-[2rem] mix-blend-multiply hidden md:flex"
        />
      </div>
    </div>
  );
};

const BlockNotFound: React.FC = () => {
  const { data } = useBlockExplorer();

  if (data?.isInvalidBlock) {
    return (
      <HeaderExplorer>
        <div className="flex flex-col gap-2 items-center border border-red-bean-50">
          <h3 className="exposure-m-italic text-secondary-700">
            {m["explorer.block.notFound.title"]()}
          </h3>
          <p className="diatype-m-medium max-w-[42.5rem] text-center text-tertiary-500 ">
            {m["explorer.block.notFound.description"]()}
          </p>
        </div>
      </HeaderExplorer>
    );
  }

  return null;
};

const BlockTable: React.FC = () => {
  const { data } = useBlockExplorer();

  if (!data?.searchBlock) return null;

  const { transactions } = data.searchBlock;

  return <TransactionsTable transactions={transactions} />;
};

export const BlockExplorer = Object.assign(BlockContainer, {
  Skeleton: BlockSkeleton,
  FutureBlock: FutureBlock,
  NotFound: BlockNotFound,
  Details: BlockDetails,
  TxTable: BlockTable,
});
