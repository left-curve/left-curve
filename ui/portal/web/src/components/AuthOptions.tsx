import { Button, IconChevronDown, IconPasskey, twMerge } from "@left-curve/applets-kit";
import { useConnectors } from "@left-curve/store-react";
import { AnimatePresence, motion } from "framer-motion";
import type React from "react";
import { useState } from "react";

import { m } from "~/paraglide/messages";

interface Props {
  action: (method: string) => void;
  isPending: boolean;
  mode: "signup" | "signin";
}

const containerVariants = {
  hidden: {},
  visible: {
    transition: {
      delayChildren: 0.1,
      staggerChildren: 0.1,
    },
  },
};

const childVariants = {
  hidden: { opacity: 0, y: -30 },
  visible: { opacity: 1, y: 0 },
};

export const AuthOptions: React.FC<Props> = ({ action, isPending, mode }) => {
  const [expandWallets, setExpandWallets] = useState(false);
  const connectors = useConnectors();

  return (
    <div className="flex flex-col gap-6 w-full">
      <Button fullWidth onClick={() => action("passkey")} isLoading={isPending} className="gap-2">
        <IconPasskey className="w-6 h-6" />
        <p className="min-w-20"> {m["common.signWithPasskey"]({ action: mode })}</p>
      </Button>
      <div className="flex items-center justify-center text-gray-500">
        <span className="flex-1 h-[1px] bg-gray-100" />
        <div
          className="flex items-center justify-center gap-1 px-2 cursor-pointer"
          onClick={() => setExpandWallets(!expandWallets)}
        >
          <p>{m["common.signWithWallet"]({ action: mode })}</p>
          <IconChevronDown
            className={twMerge(
              "w-4 h-4 transition-all duration-300",
              expandWallets ? "rotate-180" : "rotate-0",
            )}
          />
        </div>
        <span className="flex-1 h-[1px] bg-gray-100" />
      </div>
      <motion.div layout className="overflow-hidden">
        <AnimatePresence>
          {expandWallets && (
            <motion.div
              key="wallets"
              initial={{ opacity: 0, height: 0, paddingBottom: 0 }}
              animate={{ opacity: 1, height: "auto", paddingBottom: "1rem" }}
              exit={{ opacity: 0, height: 0, paddingBottom: 0 }}
              transition={{ duration: 0.2 }}
              className="flex flex-col gap-3"
            >
              <motion.div
                className="flex flex-col gap-3"
                variants={containerVariants}
                initial="hidden"
                animate="visible"
              >
                {connectors.map((connector) => {
                  if (connector.type === "passkey") return null;
                  return (
                    <motion.div key={connector.id} variants={childVariants}>
                      <Button
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
                    </motion.div>
                  );
                })}
              </motion.div>
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>
    </div>
  );
};
