import { capitalize } from "@left-curve/dango/utils";
import { ConnectorIds } from "@left-curve/store-react/types";

import { Button } from "../atoms/Button";

import type { Connector, ConnectorId } from "@left-curve/store-react/types";
import { WalletIcon } from "../icons/Wallet";

interface Props {
  mode: "signup" | "login";
  connectors: readonly Connector[];
  selectedConnector?: ConnectorId;
  onClick: (connectorId: string) => void;
}

export const ConnectorButtonOptions: React.FC<Props> = ({
  selectedConnector,
  connectors,
  mode,
  onClick,
}) => {
  const text = mode === "signup" ? "Sign up with" : "Log in with";
  return Object.values(ConnectorIds).map((connectorId) => {
    if (connectorId === "passkey") return null;
    const connector = connectors.find((connector) => connector.id === connectorId);
    const prettyName = capitalize(connectorId);
    return (
      <Button
        type="button"
        color="purple"
        key={connectorId}
        variant="bordered"
        className="flex gap-2 items-center justify-center"
        isLoading={selectedConnector === connectorId}
        isDisabled={!connector || !!selectedConnector}
        onClick={() => onClick(connectorId)}
      >
        <WalletIcon connectorId={connectorId} className="w-6 h-6 fill-typography-purple-400" />
        <span className="min-w-[12rem] text-start">
          {connector ? `${text} ${prettyName}` : `${prettyName} is not installed`}
        </span>
      </Button>
    );
  });
};
