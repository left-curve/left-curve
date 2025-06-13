import { usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";

import { AccordionItem, IconLink, twMerge } from "@left-curve/applets-kit";

import {
  AddressVisualizer,
  Badge,
  JsonVisualizer,
  TextCopy,
  createContext,
} from "@left-curve/applets-kit";
import { HeaderExplorer } from "./HeaderExplorer";

import { m } from "~/paraglide/messages";

import type { IndexedTransaction } from "@left-curve/dango/types";
import type { UseQueryResult } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";

type TransactionProps = {
  txHash: string;
  className?: string;
};

const [TransactionExplorerProvider, useTransactionExplorer] = createContext<
  UseQueryResult<IndexedTransaction | null> & { txHash: string }
>({
  strict: true,
  name: "TransactionExplorerContext",
});

const Container: React.FC<PropsWithChildren<TransactionProps>> = ({
  txHash,
  children,
  className,
}) => {
  const client = usePublicClient();
  const value = useQuery({
    queryKey: ["tx", txHash],
    queryFn: async () => {
      const txs = await client.searchTxs({ hash: txHash });
      if (!txs.nodes.length) return null;
      return txs.nodes[0];
    },
  });

  return (
    <TransactionExplorerProvider value={{ ...value, txHash }}>
      <div
        className={twMerge("w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16", className)}
      >
        {children}
      </div>
    </TransactionExplorerProvider>
  );
};

const Details: React.FC = () => {
  const navigate = useNavigate();
  const { data: tx } = useTransactionExplorer();

  if (!tx) return null;

  const { sender, hash, blockHeight, createdAt, transactionIdx, gasUsed, gasWanted, hasSucceeded } =
    tx;
  return (
    <div className="flex flex-col gap-4 rounded-xl p-4 bg-rice-25 shadow-account-card text-gray-700 diatype-sm-medium relative overflow-hidden">
      <h1 className="h4-bold">{m["explorer.txs.txDetails"]()}</h1>

      <div className="grid grid-cols-1 gap-3 md:gap-2">
        <div className="flex md:items-center gap-1 flex-col md:flex-row">
          <p className="diatype-sm-medium text-gray-500 md:min-w-[8rem]">
            {m["explorer.txs.txHash"]()}
          </p>
          <p className="break-all whitespace-normal">
            {hash}
            <TextCopy
              className="inline-block align-middle ml-1 w-4 h-4 cursor-pointer"
              copyText={hash}
            />
          </p>
        </div>
        <div className="flex md:items-center gap-1 flex-col md:flex-row">
          <p className="diatype-sm-medium text-gray-500 md:min-w-[8rem]">
            {m["explorer.txs.sender"]()}
          </p>
          <AddressVisualizer
            address={sender}
            classNames={{ text: "break-all whitespace-normal diatype-sm-bold" }}
            withIcon
            onClick={(url) => navigate({ to: url })}
          />
        </div>
        <div className="flex md:items-center gap-1 flex-col md:flex-row">
          <p className="diatype-sm-medium text-gray-500 md:min-w-[8rem]">
            {m["explorer.txs.time"]()}
          </p>
          <p className="break-all whitespace-normal">{new Date(createdAt).toLocaleString()}</p>
        </div>
        <div className="flex md:items-center gap-1 flex-col md:flex-row">
          <p className="diatype-sm-medium text-gray-500 md:min-w-[8rem]">
            {m["explorer.txs.block"]()}
          </p>
          <button
            type="button"
            className="flex gap-1 items-center"
            onClick={() => navigate({ to: `/block/${blockHeight}` })}
          >
            <span>{blockHeight}</span>
            <IconLink className="w-4 h-4" />
          </button>
        </div>
        <div className="flex md:items-center gap-1 flex-col md:flex-row">
          <p className="diatype-sm-medium text-gray-500 md:min-w-[8rem]">
            {m["explorer.txs.index"]()}
          </p>
          <p>{transactionIdx}</p>
        </div>
        <div className="flex md:items-center gap-1 flex-col md:flex-row">
          <p className="diatype-sm-medium text-gray-500 md:min-w-[8rem]">
            {m["explorer.txs.gasUsed"]()}
          </p>
          <p>{gasUsed}</p>
        </div>
        <div className="flex md:items-center gap-1 flex-col md:flex-row">
          <p className="diatype-sm-medium text-gray-500 md:min-w-[8rem]">
            {m["explorer.txs.gasWanted"]()}
          </p>
          <p>{gasWanted}</p>
        </div>
        <div className="flex md:items-center gap-1 flex-col md:flex-row">
          <p className="diatype-sm-medium text-gray-500 md:min-w-[8rem]">
            {m["explorer.txs.status"]()}
          </p>
          <div>
            <Badge
              text={hasSucceeded ? m["explorer.txs.success"]() : m["explorer.txs.failed"]()}
              color={hasSucceeded ? "green" : "red"}
              size="m"
            />
          </div>
        </div>
      </div>
      <img
        src="/images/emojis/detailed/map-explorer.svg"
        alt="map-emoji"
        className="w-[16.25rem] h-[16.25rem] opacity-40 absolute bottom-[-1rem] right-[2rem] mix-blend-multiply hidden md:block"
      />
    </div>
  );
};

const Messages: React.FC = () => {
  const { data: tx } = useTransactionExplorer();

  if (!tx) return null;

  const { nestedEvents, messages } = tx;

  return (
    <div className="flex flex-col w-full gap-6">
      <div className="w-full shadow-account-card bg-rice-25 rounded-xl p-4 flex flex-col gap-4">
        <p className="h4-bold">{m["explorer.txs.messages"]()}</p>
        {messages.map(({ data, methodName, orderIdx }) => {
          const message = data[methodName];
          return (
            <AccordionItem
              key={orderIdx}
              text={methodName}
              classNames={{ text: "capitalize" }}
              defaultExpanded
            >
              <div className="p-4 bg-gray-700 shadow-account-card  rounded-md text-white-100">
                <JsonVisualizer json={JSON.stringify(message)} collapsed={1} />
              </div>
            </AccordionItem>
          );
        })}
      </div>
      <div className="w-full shadow-account-card bg-rice-25 rounded-xl p-4 flex flex-col gap-4">
        <p className="h4-bold">{m["explorer.txs.events"]()}</p>
        <div className="p-4 bg-gray-700 shadow-account-card  rounded-md">
          <JsonVisualizer json={nestedEvents} collapsed={1} />
        </div>
      </div>
    </div>
  );
};

const NotFound: React.FC = () => {
  const { txHash, data: tx, isLoading } = useTransactionExplorer();

  if (isLoading || tx) return null;

  return (
    <div className="w-full md:max-w-[76rem] p-4">
      <HeaderExplorer>
        <div className="flex flex-col gap-2 items-center border border-red-bean-50">
          <h3 className="exposure-m-italic text-gray-700">{m["explorer.txs.notFound.title"]()}</h3>
          <p className="diatype-m-medium max-w-[42.5rem] text-center text-gray-500 ">
            {m["explorer.txs.notFound.pre"]()}
            <span className="break-all overflow-hidden underline"> {txHash}</span>{" "}
            {m["explorer.txs.notFound.description"]()}
          </p>
        </div>
      </HeaderExplorer>
    </div>
  );
};

export const TransactionExplorer = Object.assign(Container, {
  NotFound,
  Details,
  Container,
  Messages,
});
