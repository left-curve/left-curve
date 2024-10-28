"use client";

import type { Connector } from "@leftcurve/types";

import { Button, WalletIcon } from "../../";
import { twMerge } from "../../../utils";

interface Props {
  connectors: readonly Connector[];
  onSelect: (connector: Connector | undefined) => void;
  selected: Connector | undefined;
  shouldHide?: boolean;
}

export const DisplayConnectors: React.FC<Props> = ({
  connectors,
  shouldHide,
  onSelect,
  selected,
}) => {
  return (
    <div
      className={twMerge(
        "border-b border-b-gray-200 md:border-r md:border-r-gray-200 flex flex-col gap-2 p-4 transition-all overflow-hidden",
        shouldHide ? "h-0 md:h-auto md:w-0 p-0" : "",
      )}
    >
      <h1 className="text-xl font-bold pl-4 md:p-4">Connect your account</h1>
      {connectors.map((connector) => {
        return (
          <Button
            variant="flat"
            key={`connector-${connector.id}`}
            className={twMerge("p-3 md:p-4 justify-start gap-2 hover:bg-gray-100 w-full", {
              "bg-gray-100": selected?.id === connector.id,
            })}
            onClick={() => onSelect(connector)}
            size="none"
          >
            {connector.icon ? (
              <img className="h-8 w-8" src={connector.icon} alt={connector.id} />
            ) : (
              <WalletIcon connectorId={connector.id} className="h-8 w-8" />
            )}
            {connector.name}
          </Button>
        );
      })}
    </div>
  );
};
