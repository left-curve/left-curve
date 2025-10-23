import { Button } from "@left-curve/applets-kit";
import { useConnectors } from "@left-curve/store";
import { motion } from "framer-motion";
import type React from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";

interface Props {
  action: (method: string) => void;
  isPending: boolean;
}

export const AuthOptions: React.FC<Props> = ({ action, isPending }) => {
  const connectors = useConnectors();

  return (
    <div className="flex flex-col gap-4 w-full">
      {connectors.length > 2 ? (
        connectors.map((connector) => {
          if (["passkey", "session", "privy"].includes(connector.type)) return null;
          return (
            <Button
              key={connector.id}
              as={motion.div}
              isDisabled={isPending}
              className="gap-2"
              variant="secondary"
              fullWidth
              onClick={() => action(connector.id)}
            >
              <img src={connector.icon} alt={connector.name} className="w-6 h-6" />
              <p>{connector.name}</p>
            </Button>
          );
        })
      ) : (
        <p className="text-center text-primitives-blue-light-400">
          {m["common.notWalletDetected"]()}
        </p>
      )}
    </div>
  );
};
