import { usePublicClient } from "@left-curve/store";
import { type UseQueryResult, useQuery } from "@tanstack/react-query";
import React from "react";

import { AccordionItem, IconCopy, twMerge } from "@left-curve/applets-kit";
import { HeaderExplorer } from "./HeaderExplorer";

import { m } from "~/paraglide/messages";

import type { IndexedTransaction } from "@left-curve/dango/types";
import type { PropsWithChildren } from "react";

type TransactionProps = {
  txHash: string;
  className?: string;
};

const TransactionContext = React.createContext<
  (UseQueryResult<IndexedTransaction | null> & { txHash: string }) | null
>(null);

const useTransactionExplorer = () => {
  const context = React.useContext(TransactionContext);
  if (!context) {
    throw new Error("useTransactionExplorer must be used within a TransactionProvider");
  }
  return context;
};

const Container: React.FC<PropsWithChildren<TransactionProps>> = ({
  txHash,
  children,
  className,
}) => {
  const client = usePublicClient();
  const value = useQuery({
    queryKey: ["tx", txHash],
    queryFn: () => client.searchTx({ hash: txHash }),
  });

  return (
    <TransactionContext.Provider value={{ ...value, txHash }}>
      <div
        className={twMerge("w-full md:max-w-[76rem] flex flex-col gap-6 p-4 pt-6 mb-16", className)}
      >
        {children}
      </div>
    </TransactionContext.Provider>
  );
};

const Details: React.FC = () => {
  const { data: tx } = useTransactionExplorer();

  if (!tx) return null;

  const { sender, hash, blockHeight, createdAt, transactionIdx } = tx;
  return (
    <div className="flex flex-col gap-4 rounded-md px-4 py-3 bg-rice-25 shadow-card-shadow text-gray-700 diatype-m-bold relative overflow-hidden">
      <h1 className="h4-bold">Transaction Detail</h1>
      <div className="flex gap-1 items-center">
        <p className="diatype-md-medium text-gray-500">Tx hash:</p>
        <p>{hash}</p>
        <IconCopy className="w-4 h-4 text-gray-500" copyText={hash} />
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2">
        <div className="flex items-center gap-1">
          <p className="diatype-md-medium text-gray-500">Sender:</p>
          <p>{sender}</p>
        </div>
        <div className="flex items-center gap-1">
          <p className="diatype-md-medium text-gray-500">Time:</p>
          <p>{createdAt}</p>
        </div>
        <div className="flex items-center gap-1">
          <p className="diatype-md-medium text-gray-500">Block:</p>
          <p>{blockHeight}</p>
        </div>
        <div className="flex items-center gap-1">
          <p className="diatype-md-medium text-gray-500">Index:</p>
          <p>{transactionIdx}</p>
        </div>
      </div>
      <img
        src="/images/emojis/detailed/map-explorer.svg"
        alt="map-emoji"
        className="w-[16.25rem] h-[16.25rem] opacity-40 absolute top-[-2rem] right-[2rem] mix-blend-multiply "
      />
    </div>
  );
};

const Messages: React.FC = () => {
  const { data: tx } = useTransactionExplorer();

  if (!tx) return null;

  const { nestedEvents } = tx;
  return (
    <div className="w-full shadow-card-shadow bg-rice-25 rounded-3xl p-4 flex flex-col gap-4">
      <p className="h4-bold">Message</p>
      <AccordionItem text="Nested Events">
        <div className="p-4 bg-gray-700 shadow-card-shadow  rounded-md text-white-100">
          {nestedEvents}
        </div>
      </AccordionItem>
      {/* {events.length ? <p className="h4-bold">Events</p> : null}
          {events.map((event) => (
            <AccordionItem key={crypto.randomUUID()} text={event.type}>
              <div className="p-4 bg-gray-700 shadow-card-shadow  rounded-md text-white-100">
                {JSON.stringify(event.details)}
              </div>
            </AccordionItem>
          ))} */}
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
