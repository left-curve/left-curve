import { Button, ExpandOptions, IconChevronDown, IconPasskey } from "@left-curve/applets-kit";
import { useConnectors } from "@left-curve/store";
import { motion } from "framer-motion";
import type React from "react";

import { m } from "~/paraglide/messages";

interface Props {
  action: (method: string) => void;
  isPending: boolean;
  mode: "signup" | "signin";
}

export const AuthOptions: React.FC<Props> = ({ action, isPending, mode }) => {
  const connectors = useConnectors();

  return (
    <div className="flex flex-col gap-6 w-full">
      <Button fullWidth onClick={() => action("passkey")} isLoading={isPending} className="gap-2">
        <IconPasskey className="w-6 h-6" />
        <p className="min-w-20"> {m["common.signWithPasskey"]({ action: mode })}</p>
      </Button>
      <ExpandOptions showOptionText={m["common.signWithWallet"]({ action: mode })}>
        {connectors.map((connector) => {
          if (["passkey", "session"].includes(connector.type)) return null;
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
              <p className="min-w-20">{connector.name}</p>
            </Button>
          );
        })}
      </ExpandOptions>
    </div>
  );
};
