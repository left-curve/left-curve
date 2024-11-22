"use client";

import { useBlockExplorer } from "@leftcurve/react";
import { CloseIcon, ExternalLinkIcon } from "../";

interface Props {
  notification: {
    title: string;
    description: string | React.ReactNode;
    txHash: string;
  };
}

export const NotificationCard: React.FC<Props> = ({ notification }) => {
  const { title, description, txHash } = notification;
  const { getTxUrl } = useBlockExplorer();

  return (
    <div className="flex flex-col gap-2 p-2 rounded-2xl text-typography-green-500">
      <div className="flex items-center justify-between">
        <h3 className="font-extrabold text-[12px] uppercase">{title}</h3>
        <CloseIcon className="w-5 h-5" />
      </div>
      {typeof description === "string" ? <p className="text-sm">{description}</p> : description}
      <div className="flex items-center justify-between text-sm">
        <p className="uppercase text-xs  text-typography-green-400 tracking-widest font-extrabold">
          TRANSACTION HASH:
        </p>
        <div className="flex gap-1">
          <a
            href={getTxUrl(txHash)}
            target="_blank"
            rel="noopener noreferrer"
            className="flex gap-1 items-center justify-center hover:underline group"
          >
            {txHash}
            <ExternalLinkIcon className="w-5 h-5" />
          </a>
        </div>
      </div>
    </div>
  );
};
