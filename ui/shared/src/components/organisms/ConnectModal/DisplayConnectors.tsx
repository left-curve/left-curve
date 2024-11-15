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
  const passKeyConnector = connectors.find((connector) => connector.id === "passkey") as Connector;
  return (
    <div
      className={twMerge(
        "border-b border-b-gray-200 md:border-r md:border-r-gray-200 flex flex-col gap-2 p-4 transition-all overflow-hidden",
        shouldHide ? "h-0 md:h-auto md:w-0 p-0" : "",
      )}
    >
      <h3 className="text-xl font-bold pl-4 md:p-4 text-typography-rose-500">
        Connect your account
      </h3>
      <Button
        color="purple"
        variant="bordered"
        key={`connector-${passKeyConnector.id}`}
        className={twMerge("p-3 md:p-4 justify-start gap-2 w-full", {
          "bg-gray-100": selected?.id === passKeyConnector.id,
        })}
        onClick={() => onSelect(passKeyConnector)}
      >
        {passKeyConnector.icon ? (
          <img className="h-8 w-8" src={passKeyConnector.icon} alt={passKeyConnector.id} />
        ) : (
          <WalletIcon connectorId={passKeyConnector.id} className="h-8 w-8" />
        )}
        {passKeyConnector.name}
      </Button>
      <h3 className="text-xl font-bold pl-4 md:p-4 text-typography-rose-500">Other options</h3>
      {connectors.map((connector) => {
        if (connector.id === "passkey") return null;
        return (
          <Button
            color="purple"
            variant="bordered"
            key={`connector-${connector.id}`}
            className={twMerge("p-3 md:p-4 justify-start gap-2 w-full", {
              "bg-gray-100": selected?.id === connector.id,
            })}
            onClick={() => onSelect(connector)}
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
