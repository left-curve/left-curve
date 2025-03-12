import { useNavigate } from "@tanstack/react-router";

import { Command } from "cmdk";
import { AnimatePresence, motion } from "framer-motion";
import { m } from "~/paraglide/messages";
import { AppletItem } from "./AppletItem";
import { AssetItem } from "./AssetItem";

import { applets } from "../../../applets";

import type React from "react";

interface SearchMenuBodyProps {
  isVisible: boolean;
  hideMenu: () => void;
}

export const SearchMenuBody: React.FC<SearchMenuBodyProps> = ({ isVisible, hideMenu }) => {
  const navigate = useNavigate();
  return (
    <AnimatePresence mode="wait" custom={isVisible}>
      {isVisible && (
        <motion.div
          layout
          initial={{ height: 0 }}
          animate={{ height: "auto" }}
          exit={{ height: 0 }}
          transition={{ duration: 0.1 }}
          className="menu w-full overflow-hidden"
        >
          <motion.div
            className="p-1 w-full flex items-center flex-col gap-1"
            variants={{
              hidden: {},
              visible: {
                transition: {
                  delayChildren: 0.1,
                  staggerChildren: 0.05,
                },
              },
            }}
            initial="hidden"
            animate="visible"
          >
            <Command.List className="w-full">
              <Command.Empty>
                <p className="rounded-[20px] py-4 px-5 font-semibold text-[1.25rem]">
                  {m["commadBar.noResult"]()}
                </p>
              </Command.Empty>
              <Command.Group value="Applets">
                {applets.map((applet) => (
                  <Command.Item
                    key={applet.title}
                    value={applet.title}
                    className="group"
                    onSelect={() => [navigate({ to: applet.path }), hideMenu()]}
                  >
                    <AppletItem key={applet.title} {...applet} />
                  </Command.Item>
                ))}
              </Command.Group>
              {/*    <Command.Group value="Assets">
                {[].map((token) => (
                  <Command.Item
                    key={token.title}
                    value={token.title}
                    className="group"
                    onSelect={() => [navigate({ to: token.path }), hideMenu()]}
                  >
                    <TokenItem {...token} />
                  </Command.Item>
                ))}
              </Command.Group> */}
            </Command.List>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};
