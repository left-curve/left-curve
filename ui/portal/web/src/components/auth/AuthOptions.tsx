import { useState } from "react";
import { useConnectors } from "@left-curve/store";

import { Button } from "@left-curve/applets-kit";
import { motion } from "framer-motion";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import type React from "react";
interface Props {
  action: (method: string) => void;
  isPending: boolean;
}

const HIDDEN_CONNECTOR_TYPES = ["passkey", "session", "privy", "debug"];

export const AuthOptions: React.FC<Props> = ({ action, isPending }) => {
  const [selectedConnector, setSelectedConnector] = useState<string | null>(null);
  const connectors = useConnectors();

  const externalWallets = connectors.filter((c) => !HIDDEN_CONNECTOR_TYPES.includes(c.type));

  return (
    <div className="flex flex-col gap-4 w-full">
      {externalWallets.length > 0 ? (
        externalWallets.map((connector) => (
          <Button
            key={connector.id}
            as={motion.div}
            isLoading={isPending && selectedConnector === connector.id}
            isDisabled={isPending && selectedConnector !== connector.id}
            className="gap-2"
            variant="secondary"
            fullWidth
            onClick={() => [action(connector.id), setSelectedConnector(connector.id)]}
          >
            <img src={connector.icon} alt={connector.name} className="w-6 h-6" />
            <p>{connector.name}</p>
          </Button>
        ))
      ) : (
        <p className="text-center text-primitives-blue-light-400">
          {m["common.notWalletDetected"]()}
        </p>
      )}
    </div>
  );
};
